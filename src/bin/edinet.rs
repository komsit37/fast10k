use clap::{Parser, Subcommand};
use chrono::NaiveDate;
use anyhow::Result;
use tracing::{info, error};

// Reference the main library crate
use fast10k::{edinet_indexer, storage, models, downloader};

#[derive(Parser)]
#[command(name = "edinet")]
#[command(about = "EDINET command line tool")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Index commands
    Index {
        #[command(subcommand)]
        subcommand: IndexCommands,
    },
    /// Search for documents
    Search {
        /// Company ticker symbol
        #[arg(long)]
        sym: String,
    },
    /// Download documents
    Download {
        /// Company ticker symbol
        #[arg(long)]
        sym: String,

        /// Maximum number of documents to download
        #[arg(long, default_value = "5")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum IndexCommands {
    /// Show index statistics
    Stats,
    /// Update EDINET index from last date to current date
    Update,
    /// Build EDINET index from/to date
    Build {
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        from: NaiveDate,

        /// End date (YYYY-MM-DD)
        #[arg(long)]
        to: NaiveDate,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set default log level to INFO if not specified
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "edinet=info");
    }
    
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let db_path = "./fast10k.db";

    match &cli.command {
        Commands::Index { subcommand } => match subcommand {
            IndexCommands::Stats => {
                info!("Getting EDINET index statistics...");
                if let Err(e) = edinet_indexer::get_edinet_index_stats(db_path).await {
                    error!("Failed to get index statistics: {}", e);
                }
            }
            IndexCommands::Update => {
                info!("Updating EDINET index...");
                match edinet_indexer::update_edinet_index(db_path, 7).await {
                    Ok(count) => {
                        info!("Successfully updated index with {} EDINET documents", count);
                        if let Err(e) = edinet_indexer::get_edinet_index_stats(db_path).await {
                            error!("Failed to get index statistics: {}", e);
                        }
                    }
                    Err(e) => error!("EDINET index update failed: {}", e),
                }
            }
            IndexCommands::Build { from, to } => {
                info!("Building EDINET index from {} to {}...", from, to);
                match edinet_indexer::build_edinet_index_by_date(db_path, *from, *to).await {
                    Ok(count) => {
                        info!("Successfully indexed {} EDINET documents", count);
                        if let Err(e) = edinet_indexer::get_edinet_index_stats(db_path).await {
                            error!("Failed to get index statistics: {}", e);
                        }
                    }
                    Err(e) => error!("EDINET indexing failed: {}", e),
                }
            }
        },
        Commands::Search { sym } => {
            let search_query = models::SearchQuery {
                ticker: Some(sym.clone()),
                company_name: None,
                filing_type: None,
                source: Some(models::Source::Edinet),
                date_from: None,
                date_to: None,
                text_query: None,
            };
            
            match storage::search_documents(&search_query, db_path, 10).await {
                Ok(documents) => {
                    println!("date\tsym\tname\tdocType\tformats");
                    for doc in documents {
                        println!("{}\t{}\t{}\t{}\t{}", 
                            doc.date,
                            doc.ticker, 
                            doc.company_name,
                            doc.filing_type.as_str(),
                            doc.format.as_str()
                        );
                    }
                }
                Err(e) => error!("Search failed: {}", e),
            }
        }
        Commands::Download { sym, limit } => {
            info!("Downloading {} documents for symbol: {}", limit, sym);
            let download_request = models::DownloadRequest {
                source: models::Source::Edinet,
                ticker: sym.clone(),
                filing_type: None,
                date_from: None,
                date_to: None,
                limit: *limit,
                format: models::DocumentFormat::Complete,
            };
            
            match downloader::download_documents(&download_request, "./downloads").await {
                Ok(count) => info!("Successfully downloaded {} documents", count),
                Err(e) => error!("Download failed: {}", e),
            }
        }
    }

    Ok(())
}