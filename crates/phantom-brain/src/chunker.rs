//! Markdown file chunker — splits knowledge files by section heading.
//!
//! Each chunk is a semantic unit bounded by markdown headings.
//! Target size: ~500 tokens per chunk (roughly 4 chars per token).
//! Chunks preserve their source file, section heading, and line range.

use serde::{Deserialize, Serialize};

use crate::BrainError;

/// A semantic chunk extracted from a markdown knowledge file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Source knowledge file name (without path)
    pub source_file: String,
    /// Section heading (e.g., "## 4. Architecture Pattern Selection")
    pub section_heading: String,
    /// The chunk content (markdown text)
    pub content: String,
    /// Start line in the source file (1-based)
    pub line_start: usize,
    /// End line in the source file (1-based)
    pub line_end: usize,
    /// Agent roles this chunk is tagged for
    pub agent_tags: Vec<String>,
}

impl Chunk {
    /// Estimated token count (~4 chars per token).
    pub fn estimated_tokens(&self) -> usize {
        self.content.len() / 4
    }

    /// Generate a stable chunk ID from source + heading + line range.
    pub fn chunk_id(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.source_file, self.section_heading, self.line_start, self.line_end
        )
    }
}

/// Markdown chunker that splits knowledge files into semantic chunks.
pub struct MarkdownChunker {
    /// Maximum estimated tokens per chunk
    max_tokens: usize,
}

impl MarkdownChunker {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    /// Chunk a markdown file's content into semantic sections.
    ///
    /// Strategy:
    /// 1. Split on headings (lines starting with #)
    /// 2. Each heading starts a new section
    /// 3. If a section exceeds max_tokens, split on sub-headings or paragraph breaks
    /// 4. Preserve heading hierarchy for context
    pub fn chunk_file(
        &self,
        filename: &str,
        content: &str,
        agent_tags: &[String],
    ) -> Result<Vec<Chunk>, BrainError> {
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Ok(Vec::new());
        }

        let mut chunks = Vec::new();
        let mut current_heading = String::new();
        let mut current_content = String::new();
        let mut section_start_line: usize = 1;

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;

            if is_heading(line) {
                // Flush previous section if it has content
                if !current_content.trim().is_empty() {
                    let section_chunks = self.split_section(
                        filename,
                        &current_heading,
                        &current_content,
                        section_start_line,
                        line_num - 1,
                        agent_tags,
                    );
                    chunks.extend(section_chunks);
                }

                current_heading = line.to_string();
                current_content = String::new();
                current_content.push_str(line);
                current_content.push('\n');
                section_start_line = line_num;
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // Flush final section
        if !current_content.trim().is_empty() {
            let section_chunks = self.split_section(
                filename,
                &current_heading,
                &current_content,
                section_start_line,
                lines.len(),
                agent_tags,
            );
            chunks.extend(section_chunks);
        }

        Ok(chunks)
    }

    /// Split a section into chunks if it exceeds max_tokens.
    fn split_section(
        &self,
        filename: &str,
        heading: &str,
        content: &str,
        line_start: usize,
        line_end: usize,
        agent_tags: &[String],
    ) -> Vec<Chunk> {
        let estimated_tokens = content.len() / 4;

        if estimated_tokens <= self.max_tokens {
            return vec![Chunk {
                source_file: filename.to_string(),
                section_heading: heading.to_string(),
                content: content.to_string(),
                line_start,
                line_end,
                agent_tags: agent_tags.to_vec(),
            }];
        }

        // Section is too large — split on double newlines (paragraph breaks)
        let paragraphs: Vec<&str> = content.split("\n\n").collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_num = 0;

        for para in paragraphs {
            let new_size = (current_chunk.len() + para.len() + 2) / 4;

            if new_size > self.max_tokens && !current_chunk.is_empty() {
                chunk_num += 1;
                chunks.push(Chunk {
                    source_file: filename.to_string(),
                    section_heading: format!("{} (part {})", heading, chunk_num),
                    content: current_chunk.clone(),
                    line_start,
                    line_end,
                    agent_tags: agent_tags.to_vec(),
                });
                current_chunk.clear();
            }

            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(para);
        }

        if !current_chunk.trim().is_empty() {
            chunk_num += 1;
            chunks.push(Chunk {
                source_file: filename.to_string(),
                section_heading: format!(
                    "{}{}",
                    heading,
                    if chunk_num > 1 {
                        format!(" (part {})", chunk_num)
                    } else {
                        String::new()
                    }
                ),
                content: current_chunk,
                line_start,
                line_end,
                agent_tags: agent_tags.to_vec(),
            });
        }

        chunks
    }

    /// Chunk a file from disk.
    pub fn chunk_file_from_path(
        &self,
        path: &std::path::Path,
        agent_tags: &[String],
    ) -> Result<Vec<Chunk>, BrainError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| BrainError::FileReadError(format!("{}: {}", path.display(), e)))?;

        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        self.chunk_file(&filename, &content, agent_tags)
    }
}

/// Check if a line is a markdown heading.
fn is_heading(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('#') && trimmed.chars().find(|c| *c != '#') == Some(' ')
}

/// Map a knowledge file name to its default agent tags.
pub fn default_agent_tags(filename: &str) -> Vec<String> {
    match filename {
        f if f.contains("CTO_Architecture_Framework") || f.contains("CTO_Arch") => {
            vec!["cto", "architect", "security"]
                .into_iter()
                .map(String::from)
                .collect()
        }
        f if f.contains("CTO") && f.contains("Technology") => vec![
            "cto",
            "architect",
            "backend",
            "frontend",
            "devops",
            "qa",
            "security",
            "monitor",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        f if f.contains("Multi_Agent") || f.contains("Multi-Agent") => vec!["cto", "monitor"]
            .into_iter()
            .map(String::from)
            .collect(),
        f if f.contains("Build_Once") => vec!["cto", "devops", "monitor"]
            .into_iter()
            .map(String::from)
            .collect(),
        f if f.contains("Full_Stack") || f.contains("Full-Stack") => {
            vec!["backend", "frontend", "qa", "security"]
                .into_iter()
                .map(String::from)
                .collect()
        }
        f if f.contains("Every_Technology") => vec!["architect", "cto", "devops"]
            .into_iter()
            .map(String::from)
            .collect(),
        f if f.contains("Design_Expert") || f.contains("Design") => {
            vec!["frontend"].into_iter().map(String::from).collect()
        }
        f if f.contains("AI_ML") || f.contains("AI/ML") => vec!["cto", "backend", "security"]
            .into_iter()
            .map(String::from)
            .collect(),
        f if f.contains("API_Expert") || f.contains("API") => vec!["backend", "devops"]
            .into_iter()
            .map(String::from)
            .collect(),
        f if f.contains("AI_Code") && f.contains("Error") => {
            vec!["qa", "devops"].into_iter().map(String::from).collect()
        }
        _ => vec!["cto".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_heading() {
        assert!(is_heading("# Title"));
        assert!(is_heading("## Section"));
        assert!(is_heading("### Subsection"));
        assert!(is_heading("  ## Indented heading"));
        assert!(!is_heading("Not a heading"));
        assert!(!is_heading("#NoSpace"));
        assert!(!is_heading(""));
        assert!(!is_heading("```"));
    }

    #[test]
    fn test_chunk_simple_markdown() {
        let chunker = MarkdownChunker::new(500);
        let content = "\
# Title

Introduction paragraph.

## Section 1

Content of section 1.

## Section 2

Content of section 2.
More content here.
";
        let chunks = chunker
            .chunk_file("test_file", content, &["cto".to_string()])
            .unwrap();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].section_heading, "# Title");
        assert_eq!(chunks[1].section_heading, "## Section 1");
        assert_eq!(chunks[2].section_heading, "## Section 2");
    }

    #[test]
    fn test_chunk_ids_are_stable() {
        let chunker = MarkdownChunker::new(500);
        let content = "# Title\n\nSome content.\n";

        let chunks1 = chunker
            .chunk_file("test", content, &["cto".to_string()])
            .unwrap();
        let chunks2 = chunker
            .chunk_file("test", content, &["cto".to_string()])
            .unwrap();

        assert_eq!(chunks1[0].chunk_id(), chunks2[0].chunk_id());
    }

    #[test]
    fn test_large_section_is_split() {
        let chunker = MarkdownChunker::new(50); // Very small limit for testing

        // Create content that's > 200 chars (50 tokens * 4 chars/token)
        let mut content = String::from("# Big Section\n\n");
        for i in 0..20 {
            content.push_str(&format!(
                "Paragraph {} with enough content to matter.\n\n",
                i
            ));
        }

        let chunks = chunker
            .chunk_file("test", &content, &["backend".to_string()])
            .unwrap();

        assert!(
            chunks.len() > 1,
            "Large section should be split into multiple chunks"
        );
    }

    #[test]
    fn test_empty_content() {
        let chunker = MarkdownChunker::new(500);
        let chunks = chunker.chunk_file("test", "", &[]).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_agent_tags_mapping() {
        assert!(default_agent_tags("The_CTO_Architecture_Framework").contains(&"cto".to_string()));
        assert!(
            default_agent_tags("The_CTO_Architecture_Framework").contains(&"architect".to_string())
        );
        assert!(
            default_agent_tags("The_Complete_Design_Expert_Knowledge_Base")
                .contains(&"frontend".to_string())
        );
        assert!(default_agent_tags("AI_Code_GitHub_Errors_Fixes").contains(&"qa".to_string()));
        assert!(default_agent_tags("The_Complete_API_Expert_Knowledge_Base")
            .contains(&"backend".to_string()));
    }

    #[test]
    fn test_estimated_tokens() {
        let chunk = Chunk {
            source_file: "test".to_string(),
            section_heading: "# Test".to_string(),
            content: "a".repeat(400), // 400 chars = ~100 tokens
            line_start: 1,
            line_end: 1,
            agent_tags: vec![],
        };
        assert_eq!(chunk.estimated_tokens(), 100);
    }
}
