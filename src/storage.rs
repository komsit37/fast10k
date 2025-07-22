use anyhow::Result;
use sqlx::{SqlitePool, Row};
use std::path::Path;
use crate::models::{Document, SearchQuery, FilingType, Source};

pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    pub async fn new(database_path: &str) -> Result<Self> {
        // Create database if it doesn't exist
        if !Path::new(database_path).exists() {
            std::fs::File::create(database_path)?;
        }
        
        let database_url = format!("sqlite://{}", database_path);
        let pool = SqlitePool::connect(&database_url).await?;
        
        // Initialize schema
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                ticker TEXT NOT NULL,
                company_name TEXT NOT NULL,
                filing_type TEXT NOT NULL,
                source TEXT NOT NULL,
                date TEXT NOT NULL,
                content_path TEXT NOT NULL,
                metadata TEXT NOT NULL
            );
            
            CREATE INDEX IF NOT EXISTS idx_ticker ON documents(ticker);
            CREATE INDEX IF NOT EXISTS idx_date ON documents(date);
            CREATE INDEX IF NOT EXISTS idx_filing_type ON documents(filing_type);
            CREATE INDEX IF NOT EXISTS idx_source ON documents(source);
            "#
        )
        .execute(&pool)
        .await?;
        
        Ok(Storage { pool })
    }
    
    pub async fn insert_document(&self, document: &Document) -> Result<()> {
        let metadata_json = serde_json::to_string(&document.metadata)?;
        
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO documents 
            (id, ticker, company_name, filing_type, source, date, content_path, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&document.id)
        .bind(&document.ticker)
        .bind(&document.company_name)
        .bind(document.filing_type.as_str())
        .bind(document.source.as_str())
        .bind(document.date.format("%Y-%m-%d").to_string())
        .bind(document.content_path.to_string_lossy().to_string())
        .bind(&metadata_json)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn search_documents(&self, query: &SearchQuery, limit: usize) -> Result<Vec<Document>> {
        // Since dynamic parameter binding with sqlx is complex, let's use a simpler approach
        // for the initial implementation
        let rows = if let Some(ref ticker) = query.ticker {
            sqlx::query("SELECT * FROM documents WHERE ticker = ? ORDER BY date DESC LIMIT ?")
                .bind(ticker)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await?
        } else if let Some(ref company_name) = query.company_name {
            sqlx::query("SELECT * FROM documents WHERE company_name LIKE ? ORDER BY date DESC LIMIT ?")
                .bind(format!("%{}%", company_name))
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await?
        } else if let Some(ref filing_type) = query.filing_type {
            sqlx::query("SELECT * FROM documents WHERE filing_type = ? ORDER BY date DESC LIMIT ?")
                .bind(filing_type.as_str())
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await?
        } else if let Some(ref source) = query.source {
            sqlx::query("SELECT * FROM documents WHERE source = ? ORDER BY date DESC LIMIT ?")
                .bind(source.as_str())
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await?
        } else {
            // Return all documents if no specific filters
            sqlx::query("SELECT * FROM documents ORDER BY date DESC LIMIT ?")
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await?
        };
        
        let mut documents = Vec::new();
        for row in rows {
            let filing_type_str: String = row.get("filing_type");
            let source_str: String = row.get("source");
            let date_str: String = row.get("date");
            let metadata_str: String = row.get("metadata");
            
            let filing_type = match filing_type_str.as_str() {
                "10-K" => FilingType::TenK,
                "10-Q" => FilingType::TenQ,
                "8-K" => FilingType::EightK,
                "Transcript" => FilingType::Transcript,
                "Press Release" => FilingType::PressRelease,
                other => FilingType::Other(other.to_string()),
            };
            
            let source = match source_str.as_str() {
                "EDGAR" => Source::Edgar,
                "EDINET" => Source::Edinet,
                "TDNet" => Source::Tdnet,
                other => Source::Other(other.to_string()),
            };
            
            let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
            let metadata = serde_json::from_str(&metadata_str)?;
            
            documents.push(Document {
                id: row.get("id"),
                ticker: row.get("ticker"),
                company_name: row.get("company_name"),
                filing_type,
                source,
                date,
                content_path: row.get::<String, _>("content_path").into(),
                metadata,
            });
        }
        
        Ok(documents)
    }
}

// Public convenience functions
pub async fn search_documents(query: &SearchQuery, database_path: &str, limit: usize) -> Result<Vec<Document>> {
    let storage = Storage::new(database_path).await?;
    storage.search_documents(query, limit).await
}

pub async fn insert_document(document: &Document, database_path: &str) -> Result<()> {
    let storage = Storage::new(database_path).await?;
    storage.insert_document(document).await
}