//! `phantom brain` — Query and manage the Knowledge Brain.
//! `phantom brain search <query>` — Interactive TUI search panel.

use phantom_brain::knowledge::KNOWLEDGE_FILES;

use crate::dashboard;

pub async fn search(query: &str) -> anyhow::Result<()> {
    // Launch the interactive brain search TUI
    dashboard::run_brain_search(query).await
}

pub async fn update(file: &str) -> anyhow::Result<()> {
    println!("\x1b[1mKnowledge Brain Update\x1b[0m\n");

    if !std::path::Path::new(file).exists() {
        anyhow::bail!("File not found: {}", file);
    }

    println!("File: {}", file);
    println!();

    // Show known knowledge files
    println!(
        "\x1b[1;34mKnown Knowledge Files ({}):\x1b[0m",
        KNOWLEDGE_FILES.len()
    );
    for kf in KNOWLEDGE_FILES {
        println!("  - {}", kf);
    }

    println!();
    println!("\x1b[33mIngestion requires a running ChromaDB instance and embedding model.\x1b[0m");

    Ok(())
}
