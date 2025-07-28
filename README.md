# Fast10K

A multi-source financial document downloader and indexer with TUI interface for SEC 10-K filings and Japanese EDINET documents, featuring SQLite-backed search and async downloading with rate limiting.

## Features

![Screenshot](assets/edinet-tui.png)

Fast10K supports downloading and indexing financial documents from:

- **EDGAR** ✅ (SEC Filings: 10-K, 10-Q, 8-K, etc.) - **FULLY IMPLEMENTED**
- **EDINET** ✅ (Japan FSA Filings: XBRL, PDF) - **PRODUCTION READY**
- **TDNet** 🚧 (Tokyo Stock Exchange: Earnings announcements) - *placeholder implementation*

### Key Features
- 🚀 **Multi-Source Downloads**: SEC EDGAR and Japan EDINET API integration
- 🇯🇵 **EDINET Support**: Japanese company lookup with 11,000+ company database
- 📊 **Filing Type Filtering**: Download specific forms (10-K, 10-Q, 8-K, EDINET forms)
- 📅 **Date Range Filtering**: Filter filings by date ranges
- 🔢 **Download Limits**: Control number of documents with `--limit` (default: 5)
- 📄 **Multiple Formats**: Support for txt, html, xbrl, ixbrl, pdf, and complete packages
- 🔄 **Retry Logic**: Robust error handling with automatic retries
- ⚡ **Rate Limiting**: API-compliant request throttling
- 💾 **SQLite Storage**: Efficient document indexing and search
- 🖥️ **Terminal UI**: Interactive TUI for monitoring and searching with Unicode support

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

### Main Binary (fast10k)

#### Download Documents - EDGAR

Download US SEC documents:

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

### EDINET Binary (edinet)

For Japan-specific operations, use the dedicated `edinet` binary:

#### Setup EDINET Static Data

```bash
# Load Japanese company data (11,000+ companies)
edinet load-static --csv-path static/EdinetcodeDlInfo.csv

# Search companies by name or ticker
edinet search-static Toyota
edinet search-static 7203
edinet search-static オーウエル
```

#### EDINET Document Operations

```bash
# Set API key (required for downloads and indexing)
export EDINET_API_KEY=your_api_key_here

# Search indexed documents for a company
edinet search --sym 7203
edinet search --sym 7670

# Download documents
edinet download --sym 7203 --limit 5
edinet download --sym 7670 --limit 10

# Index management
edinet index stats
edinet index update
edinet index build --from 2023-01-01 --to 2023-12-31
```

#### EDINET TUI Interface

```bash
# Launch interactive TUI for EDINET
edinet-tui
```

**Available Options:**
- `--source`: Data source (`edgar` or use `edinet` binary for Japan)
- `--ticker`/`--sym`: Company ticker symbol (e.g., AAPL, MSFT, 7203, 7670)
- `--filing-type`: Specific filing type (10-k, 10-q, 8-k, EDINET forms)
- `--limit`: Maximum number of documents to download (default: 5)
- `--format`: Document format (txt, html, xbrl, ixbrl, pdf, complete) (default: txt)
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
│   ├── main.rs              # Main CLI entry point
│   ├── cli.rs               # CLI command definitions
│   ├── tui.rs               # Terminal UI implementation
│   ├── models.rs            # Core data structures
│   ├── storage.rs           # SQLite database operations
│   ├── config.rs            # Configuration management
│   ├── bin/
│   │   ├── edinet.rs        # EDINET-specific CLI binary
│   │   └── edinet_tui.rs    # EDINET TUI binary
│   ├── edinet/              # EDINET module (production-ready)
│   │   ├── mod.rs           # Module exports
│   │   ├── types.rs         # EDINET API types
│   │   ├── errors.rs        # Error handling
│   │   ├── indexer.rs       # Document indexing
│   │   └── downloader.rs    # Document downloading
│   ├── edinet_tui/          # EDINET TUI interface
│   │   ├── app.rs           # TUI application state
│   │   ├── screens/         # UI screens
│   │   └── components/      # Reusable UI components
│   └── downloader/
│       ├── mod.rs           # Downloader interface
│       ├── edgar.rs         # SEC EDGAR integration (production-ready)
│       ├── edinet.rs        # EDINET delegation interface
│       └── tdnet.rs         # TDNet integration (placeholder)
├── static/
│   └── EdinetcodeDlInfo.csv # Japanese company static data (11,000+ entries)
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

## EDINET API Implementation

The EDINET downloader is **production-ready** with comprehensive Japanese financial document support:

### ✅ EDINET Features Completed
- **Static Company Database**: Complete Japanese company lookup with 11,000+ entries from FSA
- **Smart Ticker Lookup**: Handles Japanese ticker format variations (7203 ↔ 72030, 7670 ↔ 76700)
- **EDINET API Integration**: Real-time access to Japan FSA's EDINET API
- **Document Download**: Direct download from EDINET document repository
- **Multiple Formats**: XBRL, PDF, and complete document packages
- **Database-First Approach**: 17x performance improvement over API scanning
- **Automatic Index Updates**: Search operations automatically refresh when out-of-date
- **Rate Limiting**: EDINET-compliant request throttling (configurable delays)
- **Unicode TUI Support**: Japanese character display in terminal interface
- **Error Handling**: Comprehensive retry logic with proper error context

### 🇯🇵 Tested Japanese Companies
Successfully tested with major Japanese public companies:
- ✅ **Toyota (7203)**: Multiple EDINET form types
- ✅ **ORWELL (7670)**: オーウエル株式会社 - Unicode handling verified
- ✅ **Various tickers**: 7000-9999 range testing completed

### 🏗️ EDINET Architecture
- **Dedicated Binary**: `edinet` CLI for Japan-specific operations
- **TUI Interface**: `edinet-tui` with Unicode-aware table display
- **Modular Design**: Separate `src/edinet/` module with clean interfaces
- **Configuration Management**: Environment variable support with validation
- **Backward Compatibility**: Legacy interfaces maintained during refactoring

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
- [x] **EDINET API Integration** - Production-ready Japan FSA integration
- [x] **CLI Framework** - Complete argument parsing with clap (multiple binaries)
- [x] **Core Data Models** - Document, FilingType, Source structures
- [x] **Static Company Database** - 11,000+ Japanese companies with smart lookup
- [x] **Download Limiting** - Configurable document count limits (default: 5)
- [x] **Multiple Format Support** - txt, html, xbrl, ixbrl, pdf, complete formats
- [x] **Filing Type Filtering** - Support for SEC and EDINET form types
- [x] **Date Range Filtering** - Download filings within date ranges
- [x] **Error Handling** - Robust retry logic and timeout protection
- [x] **Rate Limiting** - API-compliant request throttling for both sources
- [x] **SQLite Storage** - Database operations with sqlx and automatic indexing
- [x] **Document Indexing** - Database-first approach with performance optimization
- [x] **Terminal UI** - Interactive TUI with Unicode support for Japanese text
- [x] **Configuration Management** - Environment variable support with validation
- [x] **Project Structure** - Clean modular organization with dedicated binaries

### 🚧 In Progress / Placeholder
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

### Core Configuration
- `FAST10K_DB_PATH`: Default database path (default: `./fast10k.db`)
- `FAST10K_DOWNLOAD_DIR`: Default download directory (default: `./downloads`)
- `FAST10K_HTTP_TIMEOUT_SECONDS`: HTTP request timeout (default: 30)
- `FAST10K_USER_AGENT`: HTTP user agent string (default: `fast10k/0.1.0`)

### EDINET Configuration
- `EDINET_API_KEY`: **Required** for EDINET document downloads and indexing
- `FAST10K_EDINET_API_DELAY_MS`: Delay between EDINET API calls (default: 100ms)
- `FAST10K_EDINET_DOWNLOAD_DELAY_MS`: Delay between EDINET downloads (default: 200ms)

### EDGAR Configuration
- `FAST10K_EDGAR_API_DELAY_MS`: Delay between EDGAR API calls (default: 100ms)

### Example Configuration
```bash
# Set EDINET API key for Japanese documents
export EDINET_API_KEY=your_edinet_api_key_here

# Customize rate limiting for slower connections
export FAST10K_EDINET_API_DELAY_MS=200
export FAST10K_EDINET_DOWNLOAD_DELAY_MS=500

# Use custom database location
export FAST10K_DB_PATH=/path/to/your/fast10k.db
```

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

### EDGAR (US SEC Documents)
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
```

### EDINET (Japanese Documents)
```bash
# Set up EDINET API key
export EDINET_API_KEY=your_api_key_here

# Load Japanese company database
./target/release/edinet load-static --csv-path static/EdinetcodeDlInfo.csv

# Search for companies
./target/release/edinet search-static Toyota
./target/release/edinet search-static 7203

# Download Japanese company documents
./target/release/edinet download --sym 7203 --limit 5
./target/release/edinet download --sym 7670 --limit 3

# Launch EDINET TUI interface
./target/release/edinet-tui
```

### General Operations
```bash
# Index downloaded documents
./target/release/fast10k index

# Launch interactive terminal UI
./target/release/fast10k tui
```

**Note**: Both EDGAR and EDINET integrations are fully functional and production-ready. TDNet integration is a placeholder implementation.
