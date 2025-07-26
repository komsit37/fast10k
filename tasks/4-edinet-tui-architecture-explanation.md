# EDINET TUI Architecture Explanation

## Overview

This document provides a comprehensive explanation of how the EDINET TUI (Terminal User Interface) works, covering the architecture, event flow, search implementation, and integration points.

## Core Architecture

### 1. Event-Driven Design

The EDINET TUI operates on an asynchronous event-driven architecture:

- **Main Event Loop**: Runs continuously in `app.rs:76-97`, reading keyboard input via crossterm
- **Event Routing**: Central handler dispatches events to appropriate screen handlers
- **Global Shortcuts**: Processed at app level (ESC, Ctrl+Q, F1) before screen-specific routing
- **Non-blocking Operations**: Database and network operations run asynchronously without freezing UI

```rust
// Main event loop structure
pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
    loop {
        terminal.draw(|f| self.draw(f))?;
        
        if let Ok(event) = crossterm::event::read() {
            if let crossterm::event::Event::Key(key) = event {
                self.handle_key_event(key).await?;
            }
        }
        
        if self.should_quit { break; }
    }
}
```

### 2. Screen Management System

The TUI uses a modular screen architecture with centralized navigation:

```rust
pub enum Screen {
    MainMenu,     // Entry point with menu options
    Database,     // Index management and statistics
    Search,       // Document search form
    Results,      // Paginated search results
    Viewer,       // Document content viewer
    Help,         // Context-sensitive help
}
```

**Screen State Management**:
- Each screen maintains its own state (forms, selections, data)
- Navigation history preserved for back/forward functionality
- State transitions through `navigate_to_screen()` method
- Screen-specific event handlers for keyboard input

### 3. Application State Structure

```rust
pub struct App {
    // Navigation state
    pub current_screen: Screen,
    pub previous_screen: Option<Screen>,
    
    // Screen instances
    pub main_menu: MainMenuScreen,
    pub database: DatabaseScreen,
    pub search: SearchScreen,
    pub results: ResultsScreen,
    pub viewer: ViewerScreen,
    pub help: HelpScreen,
    
    // Global state
    pub should_quit: bool,
    pub show_help_popup: bool,
    pub status_message: Option<String>,
    pub error_message: Option<String>,
    pub config: Config,
}
```

## Search Implementation Deep Dive

### Search Form Architecture

The search screen (`search.rs`) implements a sophisticated form system:

**Field Management**:
- Enum-based field identification with Tab/Shift+Tab navigation
- Individual `InputField` instances for text inputs
- `SelectableList<FilingType>` for dropdown selections
- Focus management with visual indicators

```rust
pub enum SearchField {
    Ticker,
    CompanyName,
    FilingType,
    DateFrom,
    DateTo,
    TextQuery,
}
```

### Search Execution Flow

**1. Input Validation** (`search.rs:294-307`):
```rust
// Date format validation
if !self.date_from_input.is_empty() {
    if NaiveDate::parse_from_str(&self.date_from_input.value, "%Y-%m-%d").is_err() {
        app.set_error("Invalid 'Date From' format. Please use YYYY-MM-DD".to_string());
        return Ok(());
    }
}
```

**2. Query Construction** (`search.rs:309-326`):
```rust
let search_query = SearchQuery {
    ticker: if self.ticker_input.is_empty() { None } else { Some(self.ticker_input.value.clone()) },
    company_name: if self.company_input.is_empty() { None } else { Some(self.company_input.value.clone()) },
    filing_type: self.filing_type_list.selected().cloned(),
    source: Some(Source::Edinet),
    date_from: if self.date_from_input.is_empty() { None } else { 
        NaiveDate::parse_from_str(&self.date_from_input.value, "%Y-%m-%d").ok() 
    },
    // ... other fields
};
```

**3. Database Integration** (`search.rs:342-356`):
```rust
match storage::search_documents(&search_query, app.config.database_path_str(), 100).await {
    Ok(documents) => {
        app.set_status(format!("Found {} documents", documents.len()));
        app.results.set_documents(documents);
        self.last_query = Some(search_query);
        app.navigate_to_screen(Screen::Results);
    }
    Err(e) => {
        app.set_error(format!("Search failed: {}", e));
    }
}
```

### Search Form Features

**Input Field Capabilities**:
- Real-time character insertion and deletion
- Cursor navigation (arrows, Home/End)
- Placeholder text with visual distinction
- Focus indication through border styling
- UTF-8 safe character handling

**Filing Type Dropdown**:
- EDINET-specific document types (Annual, Quarterly, Semi-Annual, Extraordinary reports)
- Keyboard navigation (↑/↓) with visual selection
- Enter to select, ESC to cancel

**Date Input Validation**:
- YYYY-MM-DD format enforcement
- Real-time validation with immediate error feedback
- Optional date range (both from/to dates optional)

## UI Component System

### Reusable Components (`ui.rs`)

**1. InputField Component**:
```rust
pub struct InputField {
    pub label: String,
    pub value: String,
    pub placeholder: String,
    pub is_focused: bool,
    pub cursor_position: usize,
}
```

Features:
- Character insertion/deletion at cursor position
- Visual cursor rendering when focused
- Placeholder text support
- Border styling based on focus state

**2. SelectableList<T> Component**:
```rust
pub struct SelectableList<T> {
    pub items: Vec<T>,
    pub state: ListState,
}
```

Features:
- Generic type support for any list content
- Keyboard navigation (next/previous)
- Selection state management
- Integration with ratatui's ListState

**3. Styling System**:
```rust
impl Styles {
    pub fn selected() -> Style { /* Blue background, white text, bold */ }
    pub fn title() -> Style { /* Yellow text, bold */ }
    pub fn error() -> Style { /* Red text */ }
    pub fn active_border() -> Style { /* Yellow border */ }
    // ... more styles
}
```

## Results Display and Navigation

### Results Screen Architecture (`results.rs`)

**Pagination System**:
- 20 documents per page with configurable `items_per_page`
- Cross-page navigation preserves selection state
- Page boundaries handled gracefully

**Table Display Format**:
```
Date       │ Symbol    │ Company                     │ Type        │ Format
2024-03-15 │ 7203     │ Toyota Motor Corporation    │ Annual      │ Complete
2024-03-15 │ 6758     │ Sony Group Corporation      │ Quarterly   │ HTML
```

**Navigation Features**:
- ↑/↓: Item navigation within page
- PgUp/PgDn: Page navigation
- Home/End: First/last page
- Enter/v: View document
- d: Download document
- /: New search

### Document Viewer Integration

**Viewer Modes**:
1. **Info Mode**: Document metadata and filing information
2. **Content Mode**: Extracted document content with scrolling
3. **Download Mode**: Download options and progress tracking

**Content Extraction**:
- ZIP file processing for EDINET documents
- HTML parsing and text extraction
- Structured content display with navigation

## Error Handling and Validation

### Input Validation Strategy

**Real-time Validation**:
- Date format checking with immediate feedback
- Search criteria validation (at least one field required)
- File path validation for downloads

**Error Display System**:
- Status bar error messages with red styling
- Context-specific error messages
- Recovery suggestions in error text
- Non-blocking error display (operations can continue)

### Async Operation Handling

**Progress Indicators**:
- Status messages for long-running operations
- Visual feedback during search and download
- Cancellation support (ESC during operations)

**Error Recovery**:
- Graceful degradation on network failures
- Database connectivity error handling
- Timeout handling for API operations

## Integration Points

### Database Integration

**Search Operations**:
- Direct calls to `storage::search_documents()`
- SQLite-based full-text search capabilities
- Index management through existing APIs

**Statistics and Management**:
- Real-time database statistics display
- Index update and build operations
- Progress tracking for long operations

### EDINET Document Support

**Document Processing**:
- ZIP file reading and content extraction
- Support for all EDINET filing types
- Japanese text handling with proper UTF-8 boundaries

**Content Display**:
- HTML parsing for document sections
- Text extraction with formatting preservation
- Structured navigation through document parts

### Configuration Integration

**Environment Variables**:
- Uses same config system as CLI tools
- Database path: `FAST10K_DB_PATH`
- Download directory: `FAST10K_DOWNLOAD_DIR`
- API keys: `EDINET_API_KEY`
- Rate limiting: `FAST10K_*_DELAY_MS` variables

## Event Flow Diagrams

### Search Event Flow
```
User Input → Field Focus → Character Input → Form Validation → Query Build → Database Search → Results Display
     ↓              ↓              ↓               ↓              ↓              ↓              ↓
Tab/Shift+Tab   Cursor Pos    Insert Char    Date Format    SearchQuery   storage::search  ResultsScreen
```

### Navigation Event Flow
```
Key Press → Global Handler → Screen Router → Screen Handler → State Update → UI Refresh
    ↓            ↓              ↓              ↓              ↓              ↓
  ESC/F1    handle_key_event  match screen   screen.handle  update state   terminal.draw
```

## Technical Implementation Details

### Async/Await Usage

**Event Loop**:
- Main loop runs synchronously for UI responsiveness
- Database operations use async/await for non-blocking execution
- Error handling preserves async context

**Database Operations**:
```rust
match storage::search_documents(&search_query, path, limit).await {
    Ok(documents) => {
        // Handle success
    }
    Err(e) => {
        // Handle error
    }
}
```

### Memory Management

**Document Storage**:
- Documents stored in `Vec<Document>` for search results
- Pagination prevents memory issues with large result sets
- Content loaded on-demand in viewer

**State Cleanup**:
- Screen transitions clear transient state
- Error messages auto-clear on navigation
- Search results preserved until new search

### Cross-Platform Compatibility

**Terminal Support**:
- Uses crossterm for cross-platform terminal control
- Unicode support for Japanese text
- Color support with graceful fallback

**File System Integration**:
- Cross-platform path handling
- Directory creation for downloads
- File permission handling

## Performance Considerations

### Rendering Optimization

**Selective Redrawing**:
- Only redraws when state changes
- Efficient widget reuse
- Minimal allocations in render loop

**Large Dataset Handling**:
- Pagination prevents UI lag
- Lazy loading of document content
- Efficient search result caching

### Database Performance

**Query Optimization**:
- Indexed searches through SQLite
- Prepared statements for repeated queries
- Connection pooling for concurrent operations

## Future Enhancement Areas

### Planned Improvements

**Advanced Search Features**:
- Full-text search within document content
- Saved search queries
- Search history and favorites

**UI Enhancements**:
- Mouse support (optional)
- Color themes and customization
- Responsive layout for different terminal sizes

**Performance Optimizations**:
- Background index updates
- Content caching for frequently accessed documents
- Streaming for large document downloads

## Conclusion

The EDINET TUI demonstrates modern Rust TUI development practices with:

- **Clean Architecture**: Modular design with clear separation of concerns
- **Excellent UX**: Intuitive keyboard navigation and visual feedback
- **Robust Integration**: Seamless connection with existing EDINET infrastructure
- **Professional Quality**: Production-ready error handling and validation
- **Comprehensive Features**: Complete document management workflow

The implementation provides a solid foundation for Japanese financial document management and serves as a reference for building sophisticated terminal applications in Rust.