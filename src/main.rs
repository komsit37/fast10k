use clap::Parser;
use anyhow::Result;
use tracing::{info, error};

mod cli;
mod models;
mod storage;
mod indexer;
mod edinet_indexer;
mod tui;
mod downloader;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Set default log level to INFO if not specified
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "fast10k=info");
    }
    
    // Initialize logging to both console and file
    use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt, Layer};
    
    // Create a file appender for logging
    let file_appender = tracing_appender::rolling::never(".", "fast10k.log");
    
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(EnvFilter::from_default_env())
        )
        .with(
            fmt::layer()
                .with_writer(file_appender)
                .with_ansi(false)
                .with_filter(EnvFilter::from_default_env())
        )
        .init();
    
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Download { 
            source, 
            ticker, 
            filing_type, 
            from_date, 
            to_date, 
            output,
            limit,
            format
        } => {
            info!("Starting download for ticker: {}", ticker);
            
            let source = Commands::parse_source(source)?;
            let filing_type = filing_type.as_ref()
                .map(|ft| Commands::parse_filing_type(ft))
                .transpose()?;
            let document_format = Commands::parse_document_format(format)?;
                
            let download_request = models::DownloadRequest {
                source,
                ticker: ticker.clone(),
                filing_type,
                date_from: *from_date,
                date_to: *to_date,
                limit: *limit,
                format: document_format,
            };
            
            match downloader::download_documents(&download_request, output).await {
                Ok(count) => info!("Successfully downloaded {} documents", count),
                Err(e) => error!("Download failed: {}", e),
            }
        }
        
        Commands::Index { input, database } => {
            info!("Starting indexing from: {}", input);
            
            match indexer::index_documents(input, database).await {
                Ok(count) => info!("Successfully indexed {} documents", count),
                Err(e) => error!("Indexing failed: {}", e),
            }
        }
        
        Commands::Search {
            ticker,
            company,
            filing_type,
            source,
            from_date,
            to_date,
            query,
            database,
            limit,
        } => {
            let search_query = models::SearchQuery {
                ticker: ticker.clone(),
                company_name: company.clone(),
                filing_type: filing_type.as_ref()
                    .map(|ft| Commands::parse_filing_type(ft))
                    .transpose()?,
                source: source.as_ref()
                    .map(|s| Commands::parse_source(s))
                    .transpose()?,
                date_from: *from_date,
                date_to: *to_date,
                text_query: query.clone(),
            };
            
            match storage::search_documents(&search_query, database, *limit).await {
                Ok(documents) => {
                    println!("Found {} documents:", documents.len());
                    for doc in documents {
                        println!("{} - {} ({}) - {} - {}", 
                            doc.ticker, 
                            doc.company_name, 
                            doc.filing_type.as_str(),
                            doc.source.as_str(),
                            doc.date
                        );
                    }
                }
                Err(e) => error!("Search failed: {}", e),
            }
        }
        
        Commands::Tui { database } => {
            info!("Launching TUI interface");
            
            match tui::run_tui(database).await {
                Ok(_) => info!("TUI exited successfully"),
                Err(e) => error!("TUI failed: {}", e),
            }
        }
        
        
    }
    
    Ok(())
}