//! Document table component for displaying search results

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::{
    edinet_tui::ui::Styles,
    models::Document,
};

/// Configuration for document table display
#[derive(Debug, Clone)]
pub struct DocumentTableConfig {
    pub title: String,
    pub show_borders: bool,
    pub show_header: bool,
    pub max_ticker_len: usize,
    pub max_company_len: usize,
    pub max_type_len: usize,
}

impl Default for DocumentTableConfig {
    fn default() -> Self {
        Self {
            title: "Documents".to_string(),
            show_borders: true,
            show_header: true,
            max_ticker_len: 8,
            max_company_len: 15,  // reduced by 5 chars (from 20 to 15)
            max_type_len: 16,     // increased by 8 chars (from 8 to 16)
        }
    }
}

impl DocumentTableConfig {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            ..Default::default()
        }
    }

    pub fn with_column_widths(mut self, ticker: usize, company: usize, type_len: usize) -> Self {
        self.max_ticker_len = ticker;
        self.max_company_len = company;
        self.max_type_len = type_len;
        self
    }

    pub fn without_header(mut self) -> Self {
        self.show_header = false;
        self
    }
}

/// Specialized component for displaying documents in a table format
pub struct DocumentTable {
    pub documents: Vec<Document>,
    pub state: ListState,
    pub config: DocumentTableConfig,
    pub current_page: usize,
    pub items_per_page: usize,
}

impl DocumentTable {
    pub fn new(documents: Vec<Document>, config: DocumentTableConfig) -> Self {
        let mut state = ListState::default();
        if !documents.is_empty() {
            state.select(Some(0));
        }

        Self {
            documents,
            state,
            config,
            current_page: 0,
            items_per_page: 20,
        }
    }

    pub fn with_pagination(mut self, items_per_page: usize) -> Self {
        self.items_per_page = items_per_page;
        self
    }

    /// Set new documents and reset selection
    pub fn set_documents(&mut self, documents: Vec<Document>) {
        self.documents = documents;
        self.current_page = 0;
        self.state.select(if self.documents.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    /// Get documents for current page
    pub fn get_current_page_documents(&self) -> &[Document] {
        let start_idx = self.current_page * self.items_per_page;
        let end_idx = std::cmp::min(start_idx + self.items_per_page, self.documents.len());

        if start_idx < self.documents.len() {
            &self.documents[start_idx..end_idx]
        } else {
            &[]
        }
    }

    /// Get currently selected document
    pub fn get_selected_document(&self) -> Option<&Document> {
        self.state.selected().and_then(|idx| {
            let page_start = self.current_page * self.items_per_page;
            self.documents.get(page_start + idx)
        })
    }

    /// Get total number of pages
    pub fn get_total_pages(&self) -> usize {
        if self.documents.is_empty() {
            1
        } else {
            (self.documents.len() + self.items_per_page - 1) / self.items_per_page
        }
    }

    /// Navigate to next page
    pub fn next_page(&mut self) {
        if self.current_page + 1 < self.get_total_pages() {
            self.current_page += 1;
            self.state.select(Some(0)); // Reset to first item on new page
        }
    }

    /// Navigate to previous page
    pub fn previous_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.state.select(Some(0)); // Reset to first item on new page
        }
    }

    /// Navigate up within current page
    pub fn navigate_up(&mut self) {
        let page_documents = self.get_current_page_documents();
        if page_documents.is_empty() {
            return;
        }

        let selected = self.state.selected().unwrap_or(0);
        let new_selected = if selected == 0 {
            page_documents.len() - 1
        } else {
            selected - 1
        };
        self.state.select(Some(new_selected));
    }

    /// Navigate down within current page
    pub fn navigate_down(&mut self) {
        let page_documents = self.get_current_page_documents();
        if page_documents.is_empty() {
            return;
        }

        let selected = self.state.selected().unwrap_or(0);
        let new_selected = (selected + 1) % page_documents.len();
        self.state.select(Some(new_selected));
    }

    /// Render the document table
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let page_documents: Vec<_> = self.get_current_page_documents().iter().cloned().collect();
        
        let mut items = Vec::new();

        // Add header if configured
        if self.config.show_header {
            let header = Line::from(vec![
                Span::styled(
                    format!("{:<10}", "Date"),
                    Styles::title(),
                ),
                Span::styled(" | ", Styles::title()),
                Span::styled(
                    format!("{:<width$}", "Symbol", width = self.config.max_ticker_len),
                    Styles::title(),
                ),
                Span::styled(" | ", Styles::title()),
                Span::styled(
                    format!("{:<width$}", "Company", width = self.config.max_company_len),
                    Styles::title(),
                ),
                Span::styled(" | ", Styles::title()),
                Span::styled(
                    format!("{:<width$}", "Type", width = self.config.max_type_len),
                    Styles::title(),
                ),
                Span::styled(" | ", Styles::title()),
                Span::styled("Format", Styles::title()),
            ]);
            items.push(ListItem::new(header));
        }

        // Add document rows
        for (i, doc) in page_documents.iter().enumerate() {
            let style = if Some(i) == self.state.selected() {
                Styles::selected()
            } else {
                Style::default()
            };

            let content = Line::from(vec![
                Span::styled(format!("{:<10}", doc.date), style),
                Span::styled(" | ", style),
                Span::styled(
                    format!(
                        "{:<width$}",
                        doc.ticker.chars().take(self.config.max_ticker_len).collect::<String>(),
                        width = self.config.max_ticker_len
                    ),
                    style,
                ),
                Span::styled(" | ", style),
                Span::styled(
                    format!(
                        "{:<width$}",
                        doc.company_name.chars().take(self.config.max_company_len).collect::<String>(),
                        width = self.config.max_company_len
                    ),
                    style,
                ),
                Span::styled(" | ", style),
                Span::styled(
                    format!(
                        "{:<width$}",
                        doc.filing_type.as_str().chars().take(self.config.max_type_len).collect::<String>(),
                        width = self.config.max_type_len
                    ),
                    style,
                ),
                Span::styled(" | ", style),
                Span::styled(doc.format.as_str(), style),
            ]);

            items.push(ListItem::new(content));
        }

        // Add pagination info to title
        let title = if self.documents.is_empty() {
            format!("{} (Empty)", self.config.title)
        } else {
            format!(
                "{} ({}/{} - Page {}/{})",
                self.config.title,
                page_documents.len(),
                self.documents.len(),
                self.current_page + 1,
                self.get_total_pages()
            )
        };

        let block = if self.config.show_borders {
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Styles::active_border())
        } else {
            Block::default()
        };

        let list = List::new(items).block(block);

        f.render_stateful_widget(list, area, &mut self.state);
    }

    /// Render with download status indicators
    pub fn render_with_status(&mut self, f: &mut Frame, area: Rect, download_status: Option<&str>) {
        if let Some(status) = download_status {
            // Modify title to include download status
            let original_title = self.config.title.clone();
            self.config.title = format!("{} - {}", original_title, status);
            self.render(f, area);
            self.config.title = original_title;
        } else {
            self.render(f, area);
        }
    }
}