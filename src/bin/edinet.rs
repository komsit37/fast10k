use clap::{Parser, Subcommand};
use chrono::NaiveDate;
use anyhow::Result;
use tracing::{info, error};

// Reference the main library crate
use fast10k::{edinet_indexer, storage, models, downloader, config::Config};

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
    /// Load static EDINET data from CSV
    LoadStatic {
        /// Path to EdinetcodeDlInfo.csv file
        #[arg(long, default_value = "static/EdinetcodeDlInfo.csv")]
        csv_path: String,
    },
    /// Search static EDINET data
    SearchStatic {
        /// Search query (company name, symbol, or EDINET code)
        query: String,
        
        /// Maximum number of results
        #[arg(long, default_value = "20")]
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
        std::env::set_var("RUST_LOG", "edinet=info,fast10k=info");
    }
    
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let config = Config::from_env()?;
    config.validate()?;

    match &cli.command {
        Commands::Index { subcommand } => match subcommand {
            IndexCommands::Stats => {
                info!("Getting EDINET index statistics...");
                if let Err(e) = edinet_indexer::get_edinet_index_stats(config.database_path_str()).await {
                    error!("Failed to get index statistics: {}", e);
                }
            }
            IndexCommands::Update => {
                info!("Updating EDINET index...");
                match edinet_indexer::update_edinet_index(config.database_path_str(), 7).await {
                    Ok(count) => {
                        info!("Successfully updated index with {} EDINET documents", count);
                        if let Err(e) = edinet_indexer::get_edinet_index_stats(config.database_path_str()).await {
                            error!("Failed to get index statistics: {}", e);
                        }
                    }
                    Err(e) => error!("EDINET index update failed: {}", e),
                }
            }
            IndexCommands::Build { from, to } => {
                info!("Building EDINET index from {} to {}...", from, to);
                match edinet_indexer::build_edinet_index_by_date(config.database_path_str(), *from, *to).await {
                    Ok(count) => {
                        info!("Successfully indexed {} EDINET documents", count);
                        if let Err(e) = edinet_indexer::get_edinet_index_stats(config.database_path_str()).await {
                            error!("Failed to get index statistics: {}", e);
                        }
                    }
                    Err(e) => error!("EDINET indexing failed: {}", e),
                }
            }
        },
        Commands::Search { sym } => {
            // Check if index needs updating before searching
            if let Err(e) = check_and_update_index_if_needed(&config).await {
                error!("Failed to check/update index: {}", e);
            }
            
            let search_query = models::SearchQuery {
                ticker: Some(sym.clone()),
                company_name: None,
                filing_type: None,
                source: Some(models::Source::Edinet),
                date_from: None,
                date_to: None,
                text_query: None,
            };
            
            match storage::search_documents(&search_query, config.database_path_str(), 10).await {
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
            
            match downloader::download_documents(&download_request, config.download_dir_str()).await {
                Ok(count) => info!("Successfully downloaded {} documents", count),
                Err(e) => error!("Download failed: {}", e),
            }
        }
        Commands::LoadStatic { csv_path } => {
            info!("Loading EDINET static data from: {}", csv_path);
            match storage::load_edinet_static_data(config.database_path_str(), csv_path).await {
                Ok(count) => info!("Successfully loaded {} EDINET static records", count),
                Err(e) => error!("Failed to load static data: {}", e),
            }
        }
        Commands::SearchStatic { query, limit } => {
            match storage::search_edinet_static(config.database_path_str(), query, *limit).await {
                Ok(results) => {
                    println!("edinet_code\tsecurities_code\tsubmitter_name\tsubmitter_name_en\tindustry\tclosing_date\taddress");
                    for (edinet_code, submitter_name, submitter_name_en, securities_code, industry, closing_date, address) in results {
                        println!("{}\t{}\t{}\t{}\t{}\t{}\t{}", 
                            edinet_code, securities_code, submitter_name, submitter_name_en, industry, closing_date, address);
                    }
                }
                Err(e) => error!("Search failed: {}", e),
            }
        }
    }

    Ok(())
}

async fn check_and_update_index_if_needed(config: &Config) -> Result<()> {
    use chrono::{NaiveDate, Utc};
    
    // Check the latest date in the database
    match storage::get_date_range_for_source(&models::Source::Edinet, config.database_path_str()).await {
        Ok((_start, end_date_str)) => {
            // Parse the end date
            if let Ok(last_indexed_date) = NaiveDate::parse_from_str(&end_date_str, "%Y-%m-%d") {
                let today = Utc::now().date_naive();
                let days_behind = (today - last_indexed_date).num_days();
                
                if days_behind > 1 {
                    info!("Index is {} days behind (last indexed: {}). Updating...", days_behind, end_date_str);
                    match edinet_indexer::update_edinet_index(config.database_path_str(), days_behind + 1).await {
                        Ok(count) => {
                            info!("Updated index with {} EDINET documents", count);
                        }
                        Err(e) => {
                            error!("Failed to update index: {}", e);
                            return Err(e);
                        }
                    }
                } else {
                    info!("Index is up-to-date (last indexed: {})", end_date_str);
                }
            } else {
                error!("Failed to parse last indexed date: {}", end_date_str);
            }
        }
        Err(_) => {
            // Database might be empty, try to build initial index for last 7 days
            info!("No EDINET documents found in database. Building initial index for last 7 days...");
            match edinet_indexer::build_edinet_index(config.database_path_str(), 7).await {
                Ok(count) => {
                    info!("Built initial index with {} EDINET documents", count);
                }
                Err(e) => {
                    error!("Failed to build initial index: {}", e);
                    return Err(e);
                }
            }
        }
    }
    
    Ok(())
}