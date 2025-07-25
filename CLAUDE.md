# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Developer Context & Rust Explanations

The developer has experience with Scala, Java, JavaScript, and C++. When implementing features or making changes, explain relevant Rust concepts by relating them to familiar paradigms from these languages:

### Key Rust Concepts to Explain
- **Ownership & Borrowing**: Compare to C++ RAII and smart pointers, Java garbage collection
- **Pattern Matching**: Relate to Java switch expressions, but more powerful
- **Result/Option Types**: Compare to Java Optional, JavaScript nullable types
- **Traits**: Similar to Java interfaces but more powerful (like C++ concepts)
- **Async/Await**: Compare to JavaScript promises, Java CompletableFuture
- **Cargo/Crates**: Similar to npm/JavaScript modules, Maven/Gradle dependencies
- **Memory Safety**: How Rust prevents common C++ pitfalls at compile time
- **Immutability by Default**: Contrast with mutable-by-default in Java/JavaScript

### When Explaining Code Changes
- Reference equivalent patterns in Scala/Java/JavaScript/C++ when introducing new Rust features
- Highlight when Rust's compile-time checks prevent runtime errors common in other languages
- Explain why certain Rust patterns (like `Result<T, E>`) are preferred over exceptions
- Show how Rust's type system provides guarantees that runtime languages can't

## Build Commands

### Main Project
```bash
# Build main fast10k binary
cargo build --release
cargo build  # debug build

# Build edinet binary specifically  
cargo build --bin edinet --release
cargo build --bin edinet  # debug build

# Run tests
cargo test

# Run specific test
cargo test test_name

# Check code without building
cargo check
```

### Running Binaries
```bash
# Main fast10k CLI
./target/debug/fast10k [commands]
./target/release/fast10k [commands]

# EDINET-specific CLI  
./target/debug/edinet [commands]
./target/release/edinet [commands]
```

## Architecture Overview

This is a multi-source financial document downloader and indexer with two main binaries:

### 1. Main Binary (`fast10k`)
- **Entry Point**: `src/main.rs`
- **CLI**: `src/cli.rs` - Main CLI interface using clap
- **TUI**: `src/tui.rs` - Terminal UI with ratatui
- **Purpose**: General-purpose document downloader supporting multiple sources

### 2. EDINET Binary (`edinet`) 
- **Entry Point**: `src/bin/edinet.rs`
- **Purpose**: Japan-specific EDINET (Financial Services Agency) document management
- **Commands**: `index`, `search`, `download`, `load-static`, `search-static`

### Core Architecture

#### Configuration (`src/config.rs`)
- Centralized configuration management with `Config` struct
- Environment variable loading with validation
- Rate limiting and HTTP client configuration  
- Default paths and timeouts

#### Data Layer (`src/storage.rs`)
- SQLite database operations using sqlx
- Two main tables:
  - `documents`: Indexed document metadata
  - `edinet_static`: Japanese company static data from CSV (3,912+ companies)
- Functions for CRUD operations, search, and static data management

#### Models (`src/models.rs`)
- `Document`: Core document structure with ticker, filing type, source, date
- `FilingType`: Enum for document types (10-K, 10-Q, 8-K, etc.)
- `Source`: Data source enum (Edgar, Edinet, Tdnet)
- `DocumentFormat`: Format types (txt, html, xbrl, ixbrl, complete)
- `SearchQuery` and `DownloadRequest`: Request structures

#### EDINET Module (`src/edinet/`)
- **`mod.rs`**: Module exports and common functionality
- **`types.rs`**: Consolidated EDINET API types and constants
- **`errors.rs`**: EDINET-specific error handling with `thiserror`
- **`indexer.rs`**: Document indexing with configuration support
- **`downloader.rs`**: Document downloading with database-first approach

#### Downloaders (`src/downloader/`)
- **`mod.rs`**: Common downloader interface
- **`edgar.rs`**: SEC EDGAR API integration (production-ready)
- **`edinet.rs`**: Delegation interface to `edinet` module
- **`tdnet.rs`**: Tokyo Stock Exchange TDNet (placeholder)

#### Legacy Interfaces
- **`src/edinet_indexer.rs`**: Compatibility interface to `edinet::indexer`
- Maintains backward compatibility while delegating to new architecture

### Key Design Patterns

#### Static Data Management (EDINET)
The EDINET system uses a comprehensive static database approach:
1. **CSV Loading**: `load-static` command loads `static/EdinetcodeDlInfo.csv` (11,053 entries) 
2. **Smart Ticker Lookup**: Handles Japanese ticker format variations (7203 ↔ 72030, 7670 ↔ 76700)
3. **Database-First**: Removed hardcoded company mappings in favor of complete database lookup

#### Multi-Source Architecture  
- Each source (Edgar, Edinet, Tdnet) has its own downloader implementation
- Common interfaces in `models.rs` allow unified handling of different document types
- Source-specific binaries (`edinet`) for specialized workflows

#### Async/SQLite Integration
- Heavy use of `tokio` for async operations
- `sqlx` for type-safe SQLite operations  
- Rate limiting and retry logic for external API calls

## EDINET-Specific Workflows

### Static Data Setup
```bash
# Load Japanese company data from CSV
./target/debug/edinet load-static --csv-path static/EdinetcodeDlInfo.csv

# Search companies
./target/debug/edinet search-static Toyota
./target/debug/edinet search-static 7203
```

### Document Operations
```bash
# Search indexed documents
./target/debug/edinet search --sym 7203

# Download documents (requires EDINET_API_KEY)
EDINET_API_KEY=your_key ./target/debug/edinet download --sym 7203 --limit 5

# Index management
./target/debug/edinet index stats
./target/debug/edinet index update
./target/debug/edinet index build --from 2023-01-01 --to 2023-12-31
```

## Database Schema

### `documents` table
- `id`, `ticker`, `company_name`, `filing_type`, `source`, `date`
- `content_path`, `metadata`, `content_preview`, `format`

### `edinet_static` table  
- `edinet_code` (primary), `securities_code`, `submitter_name`, `submitter_name_en`
- `industry`, `account_closing_date`, `province` (address)
- Smart lookup handles ticker format variations

## Environment Variables

### Required
- `EDINET_API_KEY`: Required for EDINET document downloads and indexing

### Optional Configuration
- `FAST10K_DB_PATH`: Database path override (default: `./fast10k.db`)
- `FAST10K_DOWNLOAD_DIR`: Download directory override (default: `./downloads`)
- `FAST10K_HTTP_TIMEOUT_SECONDS`: HTTP request timeout (default: 30)
- `FAST10K_USER_AGENT`: HTTP user agent string (default: `fast10k/0.1.0`)

### Rate Limiting
- `FAST10K_EDINET_API_DELAY_MS`: Delay between EDINET API calls (default: 100ms)
- `FAST10K_EDINET_DOWNLOAD_DELAY_MS`: Delay between EDINET downloads (default: 200ms)  
- `FAST10K_EDGAR_API_DELAY_MS`: Delay between EDGAR API calls (default: 100ms)

## Implementation Status

- **EDGAR**: Production-ready with full SEC API integration
- **EDINET**: Production-ready with optimized database-first approach (17x speed improvement)
- **TDNet**: Placeholder implementation
- **Configuration**: Centralized config management with environment variable support
- **Error Handling**: Standardized error types with `thiserror` integration
- **Storage**: SQLite backend with comprehensive querying and automatic index updates

## Recent Refactoring (2025-07-25)

### Architecture Improvements
- **Consolidated EDINET types**: Removed duplicate type definitions across modules
- **Centralized configuration**: `Config` struct with environment variable loading and validation
- **Standardized error handling**: `EdinetError` enum with proper error context and chaining
- **Module reorganization**: Created dedicated `src/edinet/` module for better organization

### Performance Optimizations  
- **Database-first approach**: Company lookup uses static database instead of API scanning
- **Eliminated API fallbacks**: Removed slow 7-day API search fallback for company lookup
- **Automatic index updates**: Search command automatically updates index when out-of-date
- **Rate limiting optimization**: Configurable delays via environment variables

### Code Quality
- **Removed unused code**: Cleaned up placeholder functions and unused imports
- **Backward compatibility**: Legacy interfaces maintained while delegating to new architecture  
- **Comprehensive logging**: Enhanced progress tracking and error reporting
- **Type safety**: Proper error propagation and handling throughout the codebase
