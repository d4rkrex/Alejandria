mod commands;
mod config;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "alejandria")]
#[command(version, about = "Alejandria - Persistent Memory System for AI Agents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Store a new memory
    #[command(after_help = "Examples:\n  \
        alejandria store \"Fixed bug in auth module\" -t development -i high\n  \
        alejandria store \"API key for service X\" --topic-key service-x-key")]
    Store {
        /// Memory content
        content: String,

        /// Summary (defaults to truncated content)
        #[arg(long)]
        summary: Option<String>,

        /// Topic for categorization
        #[arg(long, short = 't', default_value = "general")]
        topic: String,

        /// Importance level (critical, high, medium, low)
        #[arg(long, short = 'i', default_value = "medium")]
        importance: String,

        /// Topic key for upsert workflow
        #[arg(long)]
        topic_key: Option<String>,
    },

    /// Search and recall memories
    #[command(after_help = "Examples:\n  \
        alejandria recall \"authentication\" -l 10\n  \
        alejandria recall \"bug fix\" -t development --min-score 0.5\n  \
        alejandria --json recall \"API\" -l 5")]
    Recall {
        /// Search query
        query: String,

        /// Maximum number of results
        #[arg(long, short = 'l', default_value = "5")]
        limit: usize,

        /// Filter by topic
        #[arg(long, short = 't')]
        topic: Option<String>,

        /// Minimum relevance score (0.0-1.0)
        #[arg(long, default_value = "0.3")]
        min_score: f32,
    },

    /// Update existing memory
    #[command(after_help = "Examples:\n  \
        alejandria update 01ABCDEF... --summary \"New summary\"\n  \
        alejandria update 01ABCDEF... --importance critical --topic security")]
    Update {
        /// Memory ID (ULID)
        id: String,

        /// New summary
        #[arg(long)]
        summary: Option<String>,

        /// New importance level
        #[arg(long)]
        importance: Option<String>,

        /// New topic
        #[arg(long)]
        topic: Option<String>,
    },

    /// Soft-delete a memory
    #[command(after_help = "Examples:\n  \
        alejandria forget 01ABCDEF...")]
    Forget {
        /// Memory ID (ULID)
        id: String,
    },

    /// List all topics
    #[command(after_help = "Examples:\n  \
        alejandria topics\n  \
        alejandria topics --min-count 5\n  \
        alejandria --json topics")]
    Topics {
        /// Minimum memory count per topic
        #[arg(long, default_value = "1")]
        min_count: usize,
    },

    /// Show memory statistics
    #[command(after_help = "Examples:\n  \
        alejandria stats\n  \
        alejandria --json stats")]
    Stats,

    /// Consolidate memories in a topic
    #[command(after_help = "Examples:\n  \
        alejandria consolidate -t development\n  \
        alejandria consolidate -t bugs --min-memories 10")]
    Consolidate {
        /// Topic to consolidate
        #[arg(long, short = 't')]
        topic: String,

        /// Minimum memories required
        #[arg(long, default_value = "5")]
        min_memories: usize,
    },

    /// Apply temporal decay to memories
    #[command(after_help = "Examples:\n  \
        alejandria decay\n  \
        alejandria decay --force")]
    Decay {
        /// Force decay even if auto_decay is disabled
        #[arg(long)]
        force: bool,
    },

    /// Batch embed existing memories
    #[command(after_help = "Examples:\n  \
        alejandria embed\n  \
        alejandria embed --batch-size 50 --skip-existing")]
    Embed {
        /// Batch size for processing
        #[arg(long, default_value = "100")]
        batch_size: usize,

        /// Skip memories that already have embeddings
        #[arg(long)]
        skip_existing: bool,
    },

    /// Export memories to file
    #[command(after_help = "Examples:\n  \
        alejandria export --format json --output export.json\n  \
        alejandria export --format csv --output export.csv --filter \"importance:high\"\n  \
        alejandria export --format markdown --output export.md --include-deleted")]
    Export {
        /// Export format (json, csv, markdown)
        #[arg(long, default_value = "json")]
        format: String,

        /// Output file path
        #[arg(long, short = 'o')]
        output: String,

        /// Filter (format: field:value, e.g., importance:high, tags:rust,async)
        #[arg(long, short = 'f')]
        filter: Option<String>,

        /// Include soft-deleted memories
        #[arg(long)]
        include_deleted: bool,
    },

    /// Import memories from file
    #[command(after_help = "Examples:\n  \
        alejandria import --input export.json --mode skip\n  \
        alejandria import --input export.csv --mode update\n  \
        alejandria import --input export.json --mode replace --dry-run")]
    Import {
        /// Input file path
        #[arg(long, short = 'i')]
        input: String,

        /// Import mode (skip, update, replace)
        #[arg(long, short = 'm', default_value = "skip")]
        mode: String,

        /// Dry run (validate without importing)
        #[arg(long)]
        dry_run: bool,
    },

    /// Memoir (knowledge graph) operations
    #[command(after_help = "Examples:\n  \
        alejandria memoir create \"Rust Concepts\" \"Core Rust programming concepts\"\n  \
        alejandria memoir list\n  \
        alejandria memoir add-concept \"Rust Concepts\" \"Ownership\" --definition \"...\"")]
    Memoir {
        #[command(subcommand)]
        command: MemoirCommands,
    },

    /// Start MCP server
    #[command(after_help = "Examples:\n  \
        alejandria serve\n  \
        alejandria serve --http\n  \
        alejandria serve --http --bind 0.0.0.0:8080")]
    Serve {
        /// Enable HTTP transport mode (default: stdio)
        #[arg(long)]
        http: bool,

        /// HTTP bind address (default: from config)
        #[arg(long)]
        bind: Option<String>,
    },
}

#[derive(Subcommand)]
enum MemoirCommands {
    /// Create a new memoir
    #[command(after_help = "Example:\n  \
        alejandria memoir create \"Rust Concepts\" \"Core Rust programming concepts\"")]
    Create {
        /// Memoir name
        name: String,

        /// Description
        description: String,
    },

    /// List all memoirs
    #[command(after_help = "Example:\n  \
        alejandria memoir list")]
    List,

    /// Show memoir graph
    #[command(after_help = "Example:\n  \
        alejandria memoir show \"Rust Concepts\"")]
    Show {
        /// Memoir name
        name: String,
    },

    /// Add a concept to a memoir
    #[command(after_help = "Example:\n  \
        alejandria memoir add-concept \"Rust Concepts\" \"Ownership\" --definition \"Rust's memory management system\"")]
    AddConcept {
        /// Memoir name
        memoir: String,

        /// Concept name
        name: String,

        /// Definition
        #[arg(long)]
        definition: String,

        /// Labels (comma-separated)
        #[arg(long)]
        labels: Option<String>,
    },

    /// Refine a concept
    #[command(after_help = "Example:\n  \
        alejandria memoir refine \"Rust Concepts\" \"Ownership\" --definition \"Updated definition\"")]
    Refine {
        /// Memoir name
        memoir: String,

        /// Concept name
        concept: String,

        /// New definition
        #[arg(long)]
        definition: Option<String>,

        /// New labels (comma-separated)
        #[arg(long)]
        labels: Option<String>,
    },

    /// Search within a memoir
    #[command(after_help = "Example:\n  \
        alejandria memoir search \"Rust Concepts\" \"ownership\"")]
    Search {
        /// Memoir name
        memoir: String,

        /// Search query
        query: String,
    },

    /// Search all memoirs
    #[command(after_help = "Example:\n  \
        alejandria memoir search-all \"lifetime\"")]
    SearchAll {
        /// Search query
        query: String,
    },

    /// Link two concepts
    #[command(after_help = "Example:\n  \
        alejandria memoir link \"Rust Concepts\" \"Ownership\" \"Borrowing\" --relation prerequisite_of")]
    Link {
        /// Memoir name
        memoir: String,

        /// Source concept name
        source: String,

        /// Target concept name
        target: String,

        /// Relation type (is_a, has_property, causes, etc.)
        #[arg(long, default_value = "related_to")]
        relation: String,
    },

    /// Inspect concept neighborhood
    #[command(after_help = "Example:\n  \
        alejandria memoir inspect \"Rust Concepts\" \"Ownership\" --depth 2")]
    Inspect {
        /// Memoir name
        memoir: String,

        /// Concept name
        concept: String,

        /// Traversal depth
        #[arg(long, default_value = "2")]
        depth: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Store {
            content,
            summary,
            topic,
            importance,
            topic_key,
        } => commands::store::run(content, summary, topic, importance, topic_key, cli.json),
        Commands::Recall {
            query,
            limit,
            topic,
            min_score,
        } => commands::recall::run(query, limit, topic, min_score, cli.json),
        Commands::Update {
            id,
            summary,
            importance,
            topic,
        } => commands::update::run(id, summary, importance, topic, cli.json),
        Commands::Forget { id } => commands::forget::run(id, cli.json),
        Commands::Topics { min_count } => commands::topics::run(min_count, cli.json),
        Commands::Stats => commands::stats::run(cli.json),
        Commands::Consolidate {
            topic,
            min_memories,
        } => commands::consolidate::run(topic, min_memories, cli.json),
        Commands::Decay { force } => commands::decay::run(force, cli.json),
        Commands::Embed {
            batch_size,
            skip_existing,
        } => commands::embed::run(batch_size, skip_existing, cli.json),
        Commands::Export {
            format,
            output,
            filter,
            include_deleted,
        } => commands::export::run(format, output, filter, include_deleted, cli.json),
        Commands::Import {
            input,
            mode,
            dry_run,
        } => commands::import::run(input, mode, dry_run, cli.json),
        Commands::Memoir { command } => match command {
            MemoirCommands::Create { name, description } => {
                commands::memoir::create(name, description, cli.json)
            }
            MemoirCommands::List => commands::memoir::list(cli.json),
            MemoirCommands::Show { name } => commands::memoir::show(name, cli.json),
            MemoirCommands::AddConcept {
                memoir,
                name,
                definition,
                labels,
            } => commands::memoir::add_concept(memoir, name, definition, labels, cli.json),
            MemoirCommands::Refine {
                memoir,
                concept,
                definition,
                labels,
            } => commands::memoir::refine(memoir, concept, definition, labels, cli.json),
            MemoirCommands::Search { memoir, query } => {
                commands::memoir::search(memoir, query, cli.json)
            }
            MemoirCommands::SearchAll { query } => commands::memoir::search_all(query, cli.json),
            MemoirCommands::Link {
                memoir,
                source,
                target,
                relation,
            } => commands::memoir::link(memoir, source, target, relation, cli.json),
            MemoirCommands::Inspect {
                memoir,
                concept,
                depth,
            } => commands::memoir::inspect(memoir, concept, depth, cli.json),
        },
        Commands::Serve { http, bind } => commands::serve::run(http, bind),
    };

    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("Error: {:#}", e);
            std::process::exit(1);
        }
    }
}
// Test comment
