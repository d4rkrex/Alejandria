mod client;

use client::{AlejandriaClient, MemoirAddConceptParams, MemoirCreateParams, MemoirLinkParams};
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

    // Create a memoir (knowledge graph)
    println!("Creating memoir: 'Rust Async Architecture'...");
    let memoir_params = MemoirCreateParams {
        name: "Rust Async Architecture".to_string(),
        description: Some("Knowledge graph of Rust async/await concepts and patterns".to_string()),
    };

    let memoir_id = client.memoir_create(memoir_params).await?;
    println!("✓ Created memoir with ID: {}\n", memoir_id);

    // Add 5 concepts in parallel using tokio::spawn
    println!("Adding 5 concepts in parallel...");

    let concepts = vec![
        ("Tokio Runtime", "The async runtime that executes futures"),
        ("Futures", "Values that represent incomplete computations"),
        ("Async/Await", "Syntax for writing asynchronous code"),
        ("Channels", "Message passing between async tasks"),
        ("Select Macro", "Waiting on multiple async operations"),
    ];

    // Spawn parallel tasks for concept creation
    let mut tasks = Vec::new();

    for (name, description) in concepts {
        let memoir_id_clone = memoir_id.clone();
        let name_owned = name.to_string();
        let desc_owned = description.to_string();

        // Clone Arc pointers to share client across tasks
        // Note: In production code, you'd want to implement Clone for AlejandriaClient
        // or use Arc<AlejandriaClient>. For this example, we'll create separate clients
        // which is acceptable for demonstration purposes.

        let server_path_clone =
            env::var("ALEJANDRIA_BIN").unwrap_or_else(|_| "alejandria".to_string());
        let db_path_clone = env::var("ALEJANDRIA_DB").unwrap_or_else(|_| {
            let mut home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            home.push_str("/.alejandria/memories.db");
            home
        });

        let task = tokio::spawn(async move {
            // Each task gets its own client connection (for simplicity)
            let task_client = AlejandriaClient::new(server_path_clone, db_path_clone).await?;
            tokio::time::sleep(Duration::from_millis(100)).await; // Let server initialize

            let params = MemoirAddConceptParams {
                memoir_id: memoir_id_clone,
                name: name_owned.clone(),
                description: Some(desc_owned),
                concept_type: Some("concept".to_string()),
            };

            let concept_id = task_client.memoir_add_concept(params).await?;
            task_client.close().await?;

            Ok::<(String, String), Box<dyn std::error::Error + Send + Sync>>((
                name_owned, concept_id,
            ))
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete and collect results
    let mut concept_ids = Vec::new();
    let mut errors = Vec::new();

    for task in tasks {
        match task.await {
            Ok(Ok((name, concept_id))) => {
                println!("✓ Added concept: {} ({})", name, concept_id);
                concept_ids.push(concept_id);
            }
            Ok(Err(e)) => {
                errors.push(format!("Concept creation error: {}", e));
            }
            Err(e) => {
                errors.push(format!("Task join error: {}", e));
            }
        }
    }

    if !errors.is_empty() {
        eprintln!(
            "\n⚠ Encountered {} errors during parallel concept creation:",
            errors.len()
        );
        for error in &errors {
            eprintln!("  - {}", error);
        }
        return Err("Failed to create all concepts".into());
    }

    println!();

    // Link concepts sequentially (each link depends on previous concepts existing)
    println!("Linking concepts sequentially...");

    if concept_ids.len() >= 5 {
        let links = vec![
            (0, 1, "is_a"),       // Tokio Runtime → Futures
            (1, 2, "related_to"), // Futures → Async/Await
            (2, 3, "related_to"), // Async/Await → Channels
            (3, 4, "related_to"), // Channels → Select Macro
        ];

        for (from_idx, to_idx, relation) in links {
            let link_params = MemoirLinkParams {
                memoir_id: memoir_id.clone(),
                from_concept_id: concept_ids[from_idx].clone(),
                to_concept_id: concept_ids[to_idx].clone(),
                relation: relation.to_string(),
            };

            client.memoir_link(link_params).await?;
            println!(
                "✓ Linked: concept {} → concept {} ({})",
                from_idx + 1,
                to_idx + 1,
                relation
            );
        }
    }

    println!("\n✓ Memoir operations example completed!");
    println!(
        "Created knowledge graph with {} concepts and 4 links",
        concept_ids.len()
    );

    // Clean shutdown
    client.close().await?;

    Ok(())
}
