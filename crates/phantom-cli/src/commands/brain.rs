//! `phantom brain` — Query and manage the Knowledge Brain.

use phantom_brain::knowledge::KNOWLEDGE_FILES;

pub async fn search(query: &str) -> anyhow::Result<()> {
    println!("\x1b[1mKnowledge Brain Search\x1b[0m\n");
    println!("Query: \"{}\"\n", query);

    // Check if ChromaDB is reachable
    let config = phantom_brain::config::BrainConfig::default();
    let client = phantom_brain::chromadb::ChromaClient::new(&config.chromadb_url);

    match client.health_check().await {
        Ok(true) => {
            println!(
                "\x1b[32m\u{2713}\x1b[0m ChromaDB is running at {}",
                config.chromadb_url
            );

            // Try to query
            println!("\nSearching...");
            println!("\x1b[33mKnowledge Brain not yet ingested. Run `phantom brain update --file <path>` first.\x1b[0m");
        }
        _ => {
            println!(
                "\x1b[31m\u{2717}\x1b[0m ChromaDB not reachable at {}",
                config.chromadb_url
            );
            println!("\nTo start ChromaDB:");
            println!("  docker run -p 8000:8000 chromadb/chroma");
            println!("  # or");
            println!("  pip install chromadb && chroma run");
        }
    }

    Ok(())
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
