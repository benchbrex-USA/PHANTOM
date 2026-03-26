//! File output parser — extracts `--- FILE: path ---` blocks from agent responses.

/// A parsed file from agent output.
#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub path: String,
    pub content: String,
}

/// Parse `--- FILE: path/to/file ---` blocks from agent output text.
///
/// Expected format:
/// ```text
/// --- FILE: src/main.rs ---
/// fn main() {
///     println!("hello");
/// }
/// --- FILE: src/lib.rs ---
/// pub fn add(a: i32, b: i32) -> i32 { a + b }
/// ```
pub fn parse_file_output(text: &str) -> Vec<ParsedFile> {
    let mut files = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_content = String::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // Check for --- FILE: path --- marker
        if let Some(rest) = trimmed.strip_prefix("--- FILE:") {
            if let Some(path) = rest.strip_suffix("---") {
                // Flush previous file
                if let Some(prev_path) = current_path.take() {
                    let content = strip_code_fences(&current_content);
                    let content = content.trim().to_string();
                    if !content.is_empty() {
                        files.push(ParsedFile {
                            path: prev_path,
                            content,
                        });
                    }
                }
                current_path = Some(path.trim().to_string());
                current_content.clear();
                continue;
            }
        }

        // Check for --- END FILE --- marker (optional terminator)
        if trimmed.starts_with("--- END FILE") {
            if let Some(prev_path) = current_path.take() {
                let content = strip_code_fences(&current_content);
                let content = content.trim().to_string();
                if !content.is_empty() {
                    files.push(ParsedFile {
                        path: prev_path,
                        content,
                    });
                }
            }
            current_content.clear();
            continue;
        }

        // Accumulate content
        if current_path.is_some() {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Flush last file
    if let Some(path) = current_path {
        let content = strip_code_fences(&current_content);
        let content = content.trim().to_string();
        if !content.is_empty() {
            files.push(ParsedFile { path, content });
        }
    }

    files
}

/// Strip markdown code fences from content.
///
/// Removes opening fences like ````python`, ````rust`, ````typescript`
/// and closing ``` markers.
fn strip_code_fences(text: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        // Skip opening code fences (```lang or just ```)
        if trimmed.starts_with("```") {
            continue;
        }
        lines.push(line);
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_file() {
        let input = r#"--- FILE: src/main.rs ---
fn main() {
    println!("hello");
}
"#;
        let files = parse_file_output(input);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/main.rs");
        assert!(files[0].content.contains("fn main()"));
    }

    #[test]
    fn test_parse_multiple_files() {
        let input = r#"--- FILE: src/main.rs ---
fn main() {}
--- FILE: src/lib.rs ---
pub fn add(a: i32, b: i32) -> i32 { a + b }
"#;
        let files = parse_file_output(input);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "src/main.rs");
        assert_eq!(files[1].path, "src/lib.rs");
    }

    #[test]
    fn test_strip_code_fences() {
        let input = r#"--- FILE: src/main.rs ---
```rust
fn main() {
    println!("hello");
}
```
"#;
        let files = parse_file_output(input);
        assert_eq!(files.len(), 1);
        assert!(!files[0].content.contains("```"));
        assert!(files[0].content.contains("fn main()"));
    }

    #[test]
    fn test_ignores_preamble() {
        let input = r#"Here is the code you requested:

--- FILE: app.py ---
print("hello")
"#;
        let files = parse_file_output(input);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "app.py");
        assert!(!files[0].content.contains("Here is"));
    }

    #[test]
    fn test_empty_content_skipped() {
        let input = r#"--- FILE: empty.txt ---
--- FILE: has_content.txt ---
content here
"#;
        let files = parse_file_output(input);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "has_content.txt");
    }
}
