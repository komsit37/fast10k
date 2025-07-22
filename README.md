# Fast10K

A fast terminal-based CLI/TUI tool for downloading, indexing, and searching SEC 10-K filings and financial documents from multiple sources.

## Features

Fast10K supports downloading and indexing financial documents from:

- **EDGAR** (SEC Filings: 10-K, 10-Q, 8-K, etc.)
- **EDINET** (Japan FSA Filings: XBRL)
- **TDNet** (Tokyo Stock Exchange: Earnings announcements)

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
# Download EDGAR filings for Apple
fast10k download --source edgar --ticker AAPL --from-date 2023-01-01 --to-date 2023-12-31

# Download specific filing type
fast10k download --source edgar --ticker MSFT --filing-type 10-k

# Specify output directory
fast10k download --source edinet --ticker 7203 --output ./my-downloads
```

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

## Development Status

This is an initial implementation with the following status:

### ✅ Completed
- [x] Basic CLI framework with clap
- [x] Core data models (Document, FilingType, Source)
- [x] SQLite database storage with sqlx
- [x] Document indexing framework
- [x] Terminal UI with ratatui
- [x] Project structure and module organization

### 🚧 In Progress / Placeholder
- [ ] EDGAR API integration (placeholder implementation)
- [ ] EDINET API integration (placeholder implementation)  
- [ ] TDNet scraping (placeholder implementation)
- [ ] PDF text extraction
- [ ] HTML/XML content parsing
- [ ] Full-text search with tantivy

### 🔮 Future Enhancements
- [ ] Vector search integration for semantic document search
- [ ] Document summarization with AI/NLP
- [ ] Export functionality (CSV, JSON, Parquet)
- [ ] Incremental updates and change detection
- [ ] Configuration file support
- [ ] Additional financial data sources

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

**Note**: This is a development version with placeholder implementations for most data source integrations. The core framework is functional, but actual document downloading requires implementation of the specific API integrations.