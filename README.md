# Fast10K

A fast terminal-based CLI/TUI tool for downloading, indexing, and searching SEC 10-K filings and financial documents from multiple sources.

## Features

Fast10K supports downloading and indexing financial documents from:

- **EDGAR** ✅ (SEC Filings: 10-K, 10-Q, 8-K, etc.) - **FULLY IMPLEMENTED**
- **EDINET** 🚧 (Japan FSA Filings: XBRL) - *placeholder implementation*
- **TDNet** 🚧 (Tokyo Stock Exchange: Earnings announcements) - *placeholder implementation*

### Key Features
- 🚀 **Fast EDGAR Downloads**: Real SEC API integration with automatic CIK lookup
- 📊 **Filing Type Filtering**: Download specific forms (10-K, 10-Q, 8-K, etc.)
- 📅 **Date Range Filtering**: Filter filings by date ranges
- 🔢 **Download Limits**: Control number of documents with `--limit` (default: 5)
- 📄 **Multiple Formats**: Support for txt, html, xbrl, ixbrl, and complete packages
- 🔄 **Retry Logic**: Robust error handling with automatic retries
- ⚡ **Rate Limiting**: SEC-compliant request throttling
- 💾 **SQLite Storage**: Efficient document indexing and search
- 🖥️ **Terminal UI**: Interactive TUI for monitoring and searching

## Installation

### Prerequisites

- Rust 1.70 or later
- SQLite 3

### Build from Source

```bash
git clone https://github.com/yourusername/fast10k.git
cd fast10k
cargo build --release
```

The binary will be available at `target/release/fast10k`.

## Usage

### Commands

#### Download Documents

Download documents from a specified source:

```bash
# Download 5 most recent EDGAR filings for Apple (default limit)
fast10k download --source edgar --ticker AAPL

# Download specific number of documents
fast10k download --source edgar --ticker MSFT --limit 10

# Download specific filing type with limit
fast10k download --source edgar --ticker TSLA --filing-type 10-k --limit 3

# Download with date range filtering
fast10k download --source edgar --ticker GOOGL --from-date 2023-01-01 --to-date 2023-12-31

# Download specific document format
fast10k download --source edgar --ticker TSLA --format html --limit 3
fast10k download --source edgar --ticker AAPL --format ixbrl --limit 2

# Specify custom output directory
fast10k download --source edgar --ticker NVDA --output ./my-downloads --limit 15
```

**Available Options:**
- `--source`: Data source (currently only `edgar` fully implemented)
- `--ticker`: Company ticker symbol (e.g., AAPL, MSFT, TSLA)
- `--filing-type`: Specific filing type (10-k, 10-q, 8-k, etc.)
- `--limit`: Maximum number of documents to download (default: 5)
- `--format`: Document format (txt, html, xbrl, ixbrl, complete) (default: txt)
- `--from-date`: Start date filter (YYYY-MM-DD)
- `--to-date`: End date filter (YYYY-MM-DD)
- `--output`: Output directory (default: ./downloads)

#### Index Documents

Index downloaded documents into SQLite database:

```bash
# Index documents from default downloads directory
fast10k index

# Index from specific directory
fast10k index --input ./my-downloads --database ./my-fast10k.db
```

#### Search Documents

Search indexed documents:

```bash
# Search by ticker
fast10k search --ticker AAPL

# Search with date range
fast10k search --ticker GOOGL --from-date 2023-01-01 --to-date 2023-06-30

# Search by filing type
fast10k search --filing-type 10-k --limit 20

# Full text search (when implemented)
fast10k search --query "revenue growth" --ticker TSLA
```

#### Terminal UI

Launch the interactive terminal interface:

```bash
# Launch TUI
fast10k tui

# Use custom database
fast10k tui --database ./my-fast10k.db
```

### TUI Controls

- **Tab / Shift+Tab**: Switch between tabs (Search, Documents, Downloads)
- **↑/↓ or j/k**: Navigate document list
- **Enter**: Execute search (in Search tab)
- **q**: Quit application

## Project Structure

```
fast10k/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── cli.rs               # CLI command definitions
│   ├── tui.rs               # Terminal UI implementation
│   ├── models.rs            # Core data structures
│   ├── storage.rs           # Database operations
│   ├── indexer.rs           # Document indexing logic
│   └── downloader/
│       ├── mod.rs           # Downloader interface
│       ├── edgar.rs         # SEC EDGAR integration
│       ├── edinet.rs        # Japan EDINET integration
│       └── tdnet.rs         # TDNet integration
├── Cargo.toml
└── README.md
```

## EDGAR API Implementation

The EDGAR downloader is **fully implemented** and production-ready with the following features:

### ✅ EDGAR Features Completed
- **Company CIK Lookup**: Automatic ticker-to-CIK resolution using SEC's company_tickers.json
- **Filing Retrieval**: Real-time access to SEC's data.sec.gov/submissions API
- **Document Download**: Direct download from SEC EDGAR archives
- **Filing Type Filtering**: Support for 10-K, 10-Q, 8-K, and other form types
- **Date Range Filtering**: Filter filings by filing date ranges  
- **Download Limits**: Configurable document count limits (default: 5)
- **Multiple Formats**: txt, html, xbrl, ixbrl, and complete package support
- **Error Handling**: Comprehensive retry logic with exponential backoff
- **Rate Limiting**: SEC-compliant request throttling (10 requests/second max)
- **Timeout Protection**: 30-second timeouts with automatic retries

### 📊 Tested Companies
Successfully tested with major US public companies:
- ✅ **Apple (AAPL)**: 50+ documents downloaded
- ✅ **Tesla (TSLA)**: 8 10-K filings from 2018-2025
- ✅ **Microsoft (MSFT)**: Multiple filing types
- ✅ **NVIDIA (NVDA)**: Recent filings with limit controls
- ✅ **Alphabet (GOOGL)**: 10-K specific filtering

## 📄 Document Formats

Fast10K supports multiple SEC document formats to meet different use cases:

### Available Formats

| Format | Description | File Extension | Use Case |
|--------|-------------|----------------|----------|
| **txt** (default) | Raw SEC filing text | `.txt` | Quick reading, grep searches, text analysis |
| **html** | Formatted HTML documents | `.htm` | Human-readable viewing with formatting |
| **xbrl** | XBRL XML data files | `.xml` | Financial data extraction, structured analysis |
| **ixbrl** | Inline XBRL documents | `.htm` | Both human-readable and machine-parseable |
| **complete** | Full filing packages | `.zip` | Comprehensive analysis with all components |

### Format Examples

```bash
# Default text format (most compatible)
fast10k download --source edgar --ticker AAPL

# HTML for better readability
fast10k download --source edgar --ticker TSLA --format html --limit 3

# XBRL for financial data extraction
fast10k download --source edgar --ticker GOOGL --format xbrl --filing-type 10-k --limit 1

# Inline XBRL (iXBRL) for hybrid documents
fast10k download --source edgar --ticker MSFT --format ixbrl --limit 2

# Complete packages with all components
fast10k download --source edgar --ticker NVDA --format complete --limit 1
```

### Format Notes

- **txt**: Contains complete filing content in plain text format
- **html**: Includes formatting, tables, and styling for better readability  
- **xbrl**: Machine-readable structured financial data in XML format
- **ixbrl**: Combines human readability with embedded structured data tags
- **complete**: ZIP packages containing all document components and exhibits

## Development Status

### ✅ Completed - Production Ready
- [x] **EDGAR API Integration** - Full implementation with real SEC APIs
- [x] **CLI Framework** - Complete argument parsing with clap
- [x] **Core Data Models** - Document, FilingType, Source structures
- [x] **Download Limiting** - Configurable document count limits (default: 5)
- [x] **Multiple Format Support** - txt, html, xbrl, ixbrl, complete formats
- [x] **Filing Type Filtering** - Support for specific SEC form types
- [x] **Date Range Filtering** - Download filings within date ranges
- [x] **Error Handling** - Robust retry logic and timeout protection
- [x] **Rate Limiting** - SEC-compliant request throttling
- [x] **SQLite Storage** - Database operations with sqlx
- [x] **Document Indexing** - Framework for organizing downloads
- [x] **Terminal UI** - Interactive TUI with ratatui
- [x] **Project Structure** - Clean modular organization

### 🚧 In Progress / Placeholder
- [ ] EDINET API integration (Japan FSA)
- [ ] TDNet scraping (Tokyo Stock Exchange)
- [ ] PDF text extraction and parsing
- [ ] HTML/XML content processing
- [ ] Full-text search with tantivy

### 🔮 Future Enhancements
- [ ] Vector search for semantic document analysis
- [ ] AI-powered document summarization
- [ ] Export functionality (CSV, JSON, Parquet)
- [ ] Incremental updates and change detection
- [ ] Configuration file support (.toml/.yaml)
- [ ] Additional financial data sources (UK, EU markets)
- [ ] Real-time filing notifications
- [ ] Web interface complement to CLI/TUI

## Configuration

Fast10K can be configured through environment variables:

- `FAST10K_DB_PATH`: Default database path (default: `./fast10k.db`)
- `FAST10K_DOWNLOAD_DIR`: Default download directory (default: `./downloads`)

## Dependencies

Key dependencies include:

- **clap**: CLI argument parsing
- **ratatui + crossterm**: Terminal UI
- **tokio**: Async runtime
- **reqwest**: HTTP client for API requests
- **sqlx**: Database operations
- **serde**: Serialization
- **chrono**: Date/time handling
- **anyhow**: Error handling

## Contributing

This project is in early development. Contributions are welcome, especially for:

1. Implementing actual API integrations for EDGAR, EDINET, and TDNet
2. Adding PDF and HTML text extraction
3. Implementing full-text search
4. Adding more financial data sources
5. Improving the TUI interface

## License

MIT License - see LICENSE file for details.

---

## Quick Start

```bash
# Clone and build
git clone https://github.com/yourusername/fast10k.git
cd fast10k
cargo build --release

# Download Apple's 5 most recent SEC filings
./target/release/fast10k download --source edgar --ticker AAPL

# Download Tesla's 10-K filings only (last 3)
./target/release/fast10k download --source edgar --ticker TSLA --filing-type 10-k --limit 3

# Download HTML format documents for better readability
./target/release/fast10k download --source edgar --ticker GOOGL --format html --limit 2

# Download XBRL format for structured financial data
./target/release/fast10k download --source edgar --ticker MSFT --format ixbrl --limit 1

# Index downloaded documents
./target/release/fast10k index

# Launch interactive terminal UI
./target/release/fast10k tui
```

**Note**: The EDGAR integration is fully functional and production-ready. EDINET and TDNet integrations are placeholder implementations that need to be completed for those specific markets.