mod client;

use client::{AlejandriaClient, ClientError, MemRecallParams, MemStoreParams};
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    let server_path = env::var("ALEJANDRIA_BIN").unwrap_or_else(|_| "alejandria".to_string());
    let db_path = env::var("ALEJANDRIA_DB").unwrap_or_else(|_| {
        let mut home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        home.push_str("/.alejandria/memories.db");
        home
    });

    println!("Initializing Alejandria client...");
    println!("Server: {}", server_path);
    println!("Database: {}\n", db_path);

    // Initialize client - this spawns the MCP server
    let client = AlejandriaClient::new(server_path, db_path).await?;

    // Give server a moment to initialize
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Store 3 example memories
    println!("Storing 3 memories...");

    let memory1 = MemStoreParams {
        content: "Rust async/await is built on top of the Future trait and provides ergonomic syntax for asynchronous programming.".to_string(),
        summary: Some("Rust async fundamentals".to_string()),
        importance: Some("high".to_string()),
        topic: Some("rust-learning".to_string()),
        topic_key: Some("rust-async-basics".to_string()),
        source: Some("example_memory.rs".to_string()),
        related_ids: None,
    };

    let id1 = client.mem_store(memory1).await?;
    println!("✓ Stored memory #1 with ID: {}", id1);

    let memory2 = MemStoreParams {
        content: "Tokio is a runtime for writing asynchronous applications with Rust. It provides I/O, networking, scheduling, timers, and more.".to_string(),
        summary: Some("Tokio runtime overview".to_string()),
        importance: Some("high".to_string()),
        topic: Some("rust-learning".to_string()),
        topic_key: Some("tokio-intro".to_string()),
        source: Some("example_memory.rs".to_string()),
        related_ids: Some(vec![id1.clone()]),
    };

    let id2 = client.mem_store(memory2).await?;
    println!("✓ Stored memory #2 with ID: {}", id2);

    let memory3 = MemStoreParams {
        content: "The Pin type in Rust ensures that values cannot move in memory, which is essential for self-referential types in async code.".to_string(),
        summary: Some("Pin and memory safety".to_string()),
        importance: Some("medium".to_string()),
        topic: Some("tokio-patterns".to_string()),
        topic_key: Some("pin-explained".to_string()),
        source: Some("example_memory.rs".to_string()),
        related_ids: Some(vec![id1.clone(), id2.clone()]),
    };

    let id3 = client.mem_store(memory3).await?;
    println!("✓ Stored memory #3 with ID: {}\n", id3);

    // Recall memories with timeout handling
    println!("Recalling memories about 'Rust async patterns'...");

    let recall_params = MemRecallParams {
        query: "Rust async patterns".to_string(),
        limit: Some(10),
        min_score: Some(0.5),
        topic: None,
    };

    // Wrap recall in timeout (10 seconds)
    match tokio::time::timeout(Duration::from_secs(10), client.mem_recall(recall_params)).await {
        Ok(Ok(memories)) => {
            println!("Found {} memories:", memories.len());
            for memory in memories {
                let summary_display = memory
                    .summary
                    .unwrap_or_else(|| memory.content.chars().take(50).collect());
                println!(
                    "  - [{}] {} (score: {:.2})",
                    memory.id, summary_display, memory.score
                );
            }
            println!();
        }
        Ok(Err(e)) => {
            eprintln!("⚠ Error recalling memories: {}", e);
            eprintln!("Note: mem_recall may have server-side issues with FTS5 syntax\n");
        }
        Err(_) => {
            eprintln!("⚠ Recall operation timed out after 10 seconds\n");
        }
    }

    // List topics
    println!("Listing topics...");
    let topics = client.mem_list_topics().await?;
    println!("Topics:");
    for topic in topics {
        println!("  - {} ({} memories)", topic.topic, topic.memory_count);
    }

    println!("\n✓ Memory operations example completed!");

    // Clean shutdown
    client.close().await?;

    Ok(())
}
