use anyhow::Result;
use sqlx::{SqlitePool, Row};
use std::path::Path;
use crate::models::{Document, SearchQuery, FilingType, Source, DocumentFormat};

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
                metadata TEXT NOT NULL,
                content_preview TEXT,
                format TEXT
            );
            
            CREATE INDEX IF NOT EXISTS idx_ticker ON documents(ticker);
            CREATE INDEX IF NOT EXISTS idx_date ON documents(date);
            CREATE INDEX IF NOT EXISTS idx_filing_type ON documents(filing_type);
            CREATE INDEX IF NOT EXISTS idx_source ON documents(source);
            CREATE INDEX IF NOT EXISTS idx_company_name ON documents(company_name);
            "#
        )
        .execute(&pool)
        .await?;
        
        Ok(Storage { pool })
    }
    
    pub async fn insert_document(&self, document: &Document) -> Result<()> {
        let metadata_json = serde_json::to_string(&document.metadata)?;
        let content_preview = document.metadata.get("content_preview").map(|s| s.as_str()).unwrap_or("");
        
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO documents 
            (id, ticker, company_name, filing_type, source, date, content_path, metadata, content_preview, format)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .bind(content_preview)
        .bind(document.format.as_str())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn search_documents(&self, query: &SearchQuery, limit: usize) -> Result<Vec<Document>> {
        // Build dynamic SQL query based on provided filters
        let mut conditions = Vec::new();
        let mut params: Vec<String> = Vec::new();
        
        if let Some(ref ticker) = query.ticker {
            conditions.push("ticker = ?");
            params.push(ticker.clone());
        }
        
        if let Some(ref company_name) = query.company_name {
            conditions.push("company_name LIKE ?");
            params.push(format!("%{}%", company_name));
        }
        
        if let Some(ref filing_type) = query.filing_type {
            conditions.push("filing_type = ?");
            params.push(filing_type.as_str().to_string());
        }
        
        if let Some(ref source) = query.source {
            conditions.push("source = ?");
            params.push(source.as_str().to_string());
        }
        
        if let Some(date_from) = query.date_from {
            conditions.push("date >= ?");
            params.push(date_from.format("%Y-%m-%d").to_string());
        }
        
        if let Some(date_to) = query.date_to {
            conditions.push("date <= ?");
            params.push(date_to.format("%Y-%m-%d").to_string());
        }
        
        if let Some(ref text_query) = query.text_query {
            conditions.push("(company_name LIKE ? OR content_preview LIKE ?)");
            params.push(format!("%{}%", text_query));
            params.push(format!("%{}%", text_query));
        }
        
        // Build the final SQL query
        let base_query = "SELECT * FROM documents";
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };
        let order_clause = " ORDER BY date DESC";
        let limit_clause = format!(" LIMIT {}", limit);
        
        let sql = format!("{}{}{}{}", base_query, where_clause, order_clause, limit_clause);
        
        // Execute query with parameters
        let mut query = sqlx::query(&sql);
        for param in &params {
            query = query.bind(param);
        }
        
        let rows = query.fetch_all(&self.pool).await?;
        
        let mut documents = Vec::new();
        for row in rows {
            let filing_type_str: String = row.get("filing_type");
            let source_str: String = row.get("source");
            let date_str: String = row.get("date");
            let metadata_str: String = row.get("metadata");
            let format_str: Option<String> = row.try_get("format").ok();
            
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
            
            let format = match format_str.as_deref() {
                Some("txt") => DocumentFormat::Txt,
                Some("html") => DocumentFormat::Html,
                Some("xbrl") => DocumentFormat::Xbrl,
                Some("ixbrl") => DocumentFormat::Ixbrl,
                Some("complete") => DocumentFormat::Complete,
                Some(other) if other.contains(',') => DocumentFormat::Other(other.to_string()),
                Some(other) => DocumentFormat::Other(other.to_string()),
                _ => DocumentFormat::Complete, // Default fallback
            };
            
            documents.push(Document {
                id: row.get("id"),
                ticker: row.get("ticker"),
                company_name: row.get("company_name"),
                filing_type,
                source,
                date,
                content_path: row.get::<String, _>("content_path").into(),
                metadata,
                format,
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

pub async fn count_documents_by_source(source: &Source, database_path: &str) -> Result<i64> {
    let storage = Storage::new(database_path).await?;
    
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE source = ?")
        .bind(source.as_str())
        .fetch_one(&storage.pool)
        .await?;
    
    Ok(count.0)
}

pub async fn get_date_range_for_source(source: &Source, database_path: &str) -> Result<(String, String)> {
    let storage = Storage::new(database_path).await?;
    
    let row = sqlx::query("SELECT MIN(date) as min_date, MAX(date) as max_date FROM documents WHERE source = ?")
        .bind(source.as_str())
        .fetch_one(&storage.pool)
        .await?;
    
    let min_date: String = row.get("min_date");
    let max_date: String = row.get("max_date");
    
    Ok((min_date, max_date))
}

pub async fn get_top_companies_for_source(source: &Source, database_path: &str, limit: usize) -> Result<Vec<(String, i64)>> {
    let storage = Storage::new(database_path).await?;
    
    let rows = sqlx::query(
        "SELECT company_name, COUNT(*) as doc_count FROM documents WHERE source = ? GROUP BY company_name ORDER BY doc_count DESC LIMIT ?"
    )
        .bind(source.as_str())
        .bind(limit as i64)
        .fetch_all(&storage.pool)
        .await?;
    
    let mut companies = Vec::new();
    for row in rows {
        let company_name: String = row.get("company_name");
        let doc_count: i64 = row.get("doc_count");
        companies.push((company_name, doc_count));
    }
    
    Ok(companies)
}