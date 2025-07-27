//! EDINET CLI binary for non-interactive search

use anyhow::Result;
use clap::{Parser, Subcommand};

use fast10k::{
    config::Config,
    models::{SearchQuery, Source},
    storage,
};

#[derive(Parser)]
#[command(name = "edinet-cli")]
#[command(about = "EDINET CLI for searching documents")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Search for documents by ticker symbol
    Search {
        /// Company ticker symbol
        #[arg(long)]
        sym: String,
    },
    /// Alias for search command
    S {
        /// Company ticker symbol
        #[arg(long)]
        sym: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Load configuration
    let config = Config::from_env()?;
    config.validate()?;

    // Handle CLI commands
    handle_command(cli.command, &config).await?;

    Ok(())
}

/// Handle CLI commands - print output and exit
async fn handle_command(command: Commands, config: &Config) -> Result<()> {
    match command {
        Commands::Search { sym } | Commands::S { sym } => {
            // Set up the search query
            let search_query = SearchQuery {
                ticker: Some(sym.clone()),
                company_name: None,
                filing_type: None,
                source: Some(Source::Edinet),
                date_from: None,
                date_to: None,
                text_query: None,
            };
            
            // Execute the search
            match storage::search_documents(&search_query, config.database_path_str(), 100).await {
                Ok(documents) => {
                    if documents.is_empty() {
                        println!("No documents found for symbol: {}", sym);
                    } else {
                        println!("Found {} documents for symbol: {}", documents.len(), sym);
                        println!();
                        println!("{:<12} {:<40} {:<15} {:<12} {:<20}", "Ticker", "Company", "Filing Type", "Date", "Path");
                        println!("{}", "-".repeat(100));
                        
                        for doc in &documents {
                            let ticker = doc.ticker.as_deref().unwrap_or("N/A");
                            let company = truncate_string(&doc.company_name, 38);
                            let filing_type = doc.filing_type.map_or("N/A".to_string(), |ft| format!("{:?}", ft));
                            let date = doc.date.format("%Y-%m-%d").to_string();
                            let path = doc.content_path.as_deref().unwrap_or("N/A");
                            
                            println!("{:<12} {:<40} {:<15} {:<12} {:<20}", 
                                ticker, company, filing_type, date, path);
                        }
                        
                        println!();
                        println!("Total: {} documents", documents.len());
                    }
                }
                Err(e) => {
                    eprintln!("Search failed for symbol {}: {}", sym, e);
                    std::process::exit(1);
                }
            }
        }
    }
    
    Ok(())
}

/// Truncate string to specified length with ellipsis
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}