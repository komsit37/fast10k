//! Search results screen for the EDINET TUI

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use unicode_width::{UnicodeWidthStr, UnicodeWidthChar};

use crate::{
    downloader,
    edinet_tui::{app::Screen, ui::Styles},
    models::{Document, DocumentFormat, DownloadRequest, Source},
};

/// Results screen state
pub struct ResultsScreen {
    pub documents: Vec<Document>,
    pub document_state: ListState,
    pub current_page: usize,
    pub items_per_page: usize,
    pub is_downloading: bool,
    pub download_status: Option<String>,
}

impl ResultsScreen {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            document_state: ListState::default(),
            current_page: 0,
            items_per_page: 20,
            is_downloading: false,
            download_status: None,
        }
    }

    /// Set new documents from search results
    pub fn set_documents(&mut self, documents: Vec<Document>) {
        self.documents = documents;
        self.current_page = 0;
        self.document_state.select(if self.documents.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    /// Get current page of documents
    fn get_current_page_documents(&self) -> Vec<&Document> {
        let start_idx = self.current_page * self.items_per_page;
        let end_idx = std::cmp::min(start_idx + self.items_per_page, self.documents.len());

        if start_idx < self.documents.len() {
            self.documents[start_idx..end_idx].iter().collect()
        } else {
            Vec::new()
        }
    }

    /// Get total number of pages
    fn get_total_pages(&self) -> usize {
        if self.documents.is_empty() {
            0
        } else {
            (self.documents.len() + self.items_per_page - 1) / self.items_per_page
        }
    }

    /// Get currently selected document
    pub fn get_selected_document(&self) -> Option<&Document> {
        self.document_state.selected().and_then(|idx| {
            let page_start = self.current_page * self.items_per_page;
            self.documents.get(page_start + idx)
        })
    }

    /// Handle key events for the results screen
    pub async fn handle_event(
        &mut self,
        key: KeyEvent,
        app: &mut super::super::app::App,
    ) -> Result<()> {
        if self.is_downloading {
            // Only allow cancellation during download
            if let KeyCode::Esc = key.code {
                self.is_downloading = false;
                self.download_status = None;
                app.set_status("Download cancelled".to_string());
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Up => {
                self.navigate_up();
            }
            KeyCode::Down => {
                self.navigate_down();
            }
            KeyCode::Left => {
                self.previous_page();
            }
            KeyCode::Right => {
                self.next_page();
            }
            KeyCode::Home => {
                self.go_to_first_page();
            }
            KeyCode::End => {
                self.go_to_last_page();
            }
            KeyCode::Enter => {
                // View selected document
                if let Some(document) = self.get_selected_document() {
                    app.viewer.set_document(document.clone());
                    app.navigate_to_screen(Screen::Viewer);
                }
            }
            KeyCode::Char('d') => {
                // Download selected document
                if let Some(document) = self.get_selected_document() {
                    self.download_document(document.clone(), app).await?;
                }
            }
            KeyCode::Char('r') => {
                // Refresh/re-execute last search
                app.set_status("Refresh functionality not implemented yet".to_string());
            }
            KeyCode::Char('/') => {
                // New search
                app.navigate_to_screen(Screen::Search);
            }
            KeyCode::Char('v') => {
                // View document (same as Enter)
                if let Some(document) = self.get_selected_document() {
                    app.viewer.set_document(document.clone());
                    app.navigate_to_screen(Screen::Viewer);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn navigate_up(&mut self) {
        let page_documents = self.get_current_page_documents();
        if page_documents.is_empty() {
            return;
        }

        let current_selection = self.document_state.selected().unwrap_or(0);
        if current_selection > 0 {
            self.document_state.select(Some(current_selection - 1));
        } else if self.current_page > 0 {
            // Go to previous page, last item
            self.current_page -= 1;
            let new_page_documents = self.get_current_page_documents();
            if !new_page_documents.is_empty() {
                self.document_state
                    .select(Some(new_page_documents.len() - 1));
            }
        }
    }

    pub fn navigate_down(&mut self) {
        let page_documents = self.get_current_page_documents();
        if page_documents.is_empty() {
            return;
        }

        let current_selection = self.document_state.selected().unwrap_or(0);
        if current_selection < page_documents.len() - 1 {
            self.document_state.select(Some(current_selection + 1));
        } else if self.current_page < self.get_total_pages() - 1 {
            // Go to next page, first item
            self.current_page += 1;
            self.document_state.select(Some(0));
        }
    }

    pub fn next_page(&mut self) {
        if self.current_page < self.get_total_pages() - 1 {
            self.current_page += 1;
            self.document_state.select(Some(0));
        }
    }

    pub fn previous_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.document_state.select(Some(0));
        }
    }

    pub fn go_to_first_page(&mut self) {
        self.current_page = 0;
        self.document_state.select(if self.documents.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    pub fn go_to_last_page(&mut self) {
        if self.get_total_pages() > 0 {
            self.current_page = self.get_total_pages() - 1;
            let page_documents = self.get_current_page_documents();
            self.document_state.select(if page_documents.is_empty() {
                None
            } else {
                Some(0)
            });
        }
    }

    /// Download selected document
    pub async fn download_document(
        &mut self,
        document: Document,
        app: &mut super::super::app::App,
    ) -> Result<()> {
        self.is_downloading = true;
        self.download_status = Some(format!("Downloading {}...", document.ticker));

        app.set_status(format!("Starting download for {}", document.ticker));

        let download_request = DownloadRequest {
            source: Source::Edinet,
            ticker: document.ticker.clone(),
            filing_type: Some(document.filing_type.clone()),
            date_from: Some(document.date),
            date_to: Some(document.date),
            limit: 1,
            format: DocumentFormat::Complete,
        };

        match downloader::download_documents(&download_request, app.config.download_dir_str()).await
        {
            Ok(count) => {
                app.set_status(format!(
                    "Successfully downloaded {} document(s) to {}",
                    count,
                    app.config.download_dir_str()
                ));
            }
            Err(e) => {
                app.set_error(format!("Download failed: {}", e));
            }
        }

        self.is_downloading = false;
        self.download_status = None;
        Ok(())
    }

    /// Draw the results screen
    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title with stats
                Constraint::Min(0),    // Results list
                Constraint::Length(4), // Instructions and pagination
            ])
            .split(area);

        // Calculate items per page based on available height
        // Subtract 3 for borders (top, bottom, header)
        let available_height = chunks[1].height.saturating_sub(3);
        let calculated_items_per_page = (available_height as usize).saturating_sub(1).max(10); // At least 10 items
        
        // Update items_per_page if it's significantly different
        if calculated_items_per_page != self.items_per_page {
            let old_page = self.current_page;
            let old_selected = self.document_state.selected();
            let old_items_per_page = self.items_per_page;
            
            self.items_per_page = calculated_items_per_page;
            
            // Recalculate current page to maintain selection position
            if let Some(selected_local_idx) = old_selected {
                let global_idx = old_page * old_items_per_page + selected_local_idx;
                self.current_page = global_idx / self.items_per_page;
                let new_local_idx = global_idx % self.items_per_page;
                self.document_state.select(Some(new_local_idx));
            }
        }

        // Draw title and stats
        self.draw_title(f, chunks[0]);

        // Draw results list
        self.draw_results_list(f, chunks[1]);

        // Draw instructions and pagination
        self.draw_bottom_info(f, chunks[2]);

        // Draw download status if downloading
        if self.is_downloading {
            self.draw_download_status(f, area);
        }
    }

    fn draw_title(&self, f: &mut Frame, area: Rect) {
        let title_text = if self.documents.is_empty() {
            "Search Results - No documents found".to_string()
        } else {
            format!("Search Results - {} documents found", self.documents.len())
        };

        let title = Paragraph::new(title_text)
            .style(Styles::title())
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn draw_results_list(&mut self, f: &mut Frame, area: Rect) {
        let page_documents = self.get_current_page_documents();

        if page_documents.is_empty() {
            let empty_message = if self.documents.is_empty() {
                "No documents found. Try adjusting your search criteria."
            } else {
                "No documents on this page."
            };

            let empty_widget = Paragraph::new(empty_message)
                .style(Styles::inactive())
                .block(
                    Block::default()
                        .title("Results")
                        .borders(Borders::ALL)
                        .border_style(Styles::inactive_border()),
                );
            f.render_widget(empty_widget, area);
            return;
        }

        // Create header
        let header = ListItem::new(Line::from(vec![
            Span::styled("No.  ", Styles::title()),
            Span::styled("│ Date       ", Styles::title()),
            Span::styled("│ Symbol   ", Styles::title()),
            Span::styled("│ Company              ", Styles::title()),  // reduced by 5 chars
            Span::styled("│ Type                ", Styles::title()),   // increased by 8 chars
            Span::styled("│ Format     ", Styles::title()),
        ]));

        // Create document items
        let items: Vec<ListItem> = std::iter::once(header)
            .chain(page_documents.iter().enumerate().map(|(i, doc)| {
                let style = if Some(i) == self.document_state.selected() {
                    Styles::selected()
                } else {
                    Style::default()
                };

                let row_number = self.current_page * self.items_per_page + i + 1;
                let content = format!(
                    "{:4} │ {} │ {} │ {} │ {} │ {}",
                    row_number,
                    doc.date,
                    truncate_string(&doc.ticker, 8),
                    truncate_string(&doc.company_name, 20),
                    truncate_string(doc.filing_type.as_str(), 19),
                    truncate_string(doc.format.as_str(), 10)
                );

                ListItem::new(Line::from(Span::styled(content, style)))
            }))
            .collect();

        let results_list = List::new(items).block(
            Block::default()
                .title("Results")
                .borders(Borders::ALL)
                .border_style(Styles::active_border()),
        );

        f.render_stateful_widget(results_list, area, &mut self.document_state);
    }

    fn draw_bottom_info(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        // Instructions
        let instructions = vec![
            Line::from("↑/↓: Navigate | ←/→: Pages | Enter/v: View | d: Download"),
            Line::from("/: New Search | r: Refresh | ESC: Back"),
        ];

        let instructions_widget = Paragraph::new(instructions).style(Styles::info()).block(
            Block::default()
                .title("Instructions")
                .borders(Borders::ALL)
                .border_style(Styles::inactive_border()),
        );

        f.render_widget(instructions_widget, chunks[0]);

        // Pagination info
        let current_page = self.current_page + 1;
        let total_pages = self.get_total_pages();
        let selected_idx = self
            .document_state
            .selected()
            .map(|idx| self.current_page * self.items_per_page + idx + 1)
            .unwrap_or(0);

        let pagination_text = if total_pages > 0 {
            format!(
                "Page {} of {}\nItem {} of {}",
                current_page,
                total_pages,
                selected_idx,
                self.documents.len()
            )
        } else {
            "No pages".to_string()
        };

        let pagination_widget = Paragraph::new(pagination_text).style(Styles::info()).block(
            Block::default()
                .title("Navigation")
                .borders(Borders::ALL)
                .border_style(Styles::inactive_border()),
        );

        f.render_widget(pagination_widget, chunks[1]);
    }

    fn draw_download_status(&self, f: &mut Frame, area: Rect) {
        use crate::edinet_tui::ui::centered_rect;

        let popup_area = centered_rect(50, 20, area);

        let default_status = "Downloading...".to_string();
        let status_text = self.download_status.as_ref().unwrap_or(&default_status);

        let status_widget = Paragraph::new(format!("{}\n\nPress ESC to cancel", status_text))
            .style(Styles::info())
            .block(
                Block::default()
                    .title("Download Status")
                    .borders(Borders::ALL)
                    .border_style(Styles::warning()),
            );

        f.render_widget(ratatui::widgets::Clear, popup_area);
        f.render_widget(status_widget, popup_area);
    }
}

/// Helper function to truncate strings to a specific display width (Unicode-aware)
fn truncate_string(s: &str, max_width: usize) -> String {
    let display_width = s.width();
    if display_width <= max_width {
        // Pad with spaces to reach exact width
        let padding = max_width - display_width;
        format!("{}{}", s, " ".repeat(padding))
    } else {
        // Truncate by character until we fit within max_width - 1 (for ellipsis)
        let target_width = max_width.saturating_sub(1);
        let mut truncated = String::new();
        let mut current_width = 0;
        
        for ch in s.chars() {
            let ch_width = ch.width().unwrap_or(0);
            if current_width + ch_width > target_width {
                break;
            }
            truncated.push(ch);
            current_width += ch_width;
        }
        
        // Add ellipsis and pad to exact width
        let ellipsis_width = 1;
        let padding_needed = max_width - current_width - ellipsis_width;
        format!("{}…{}", truncated, " ".repeat(padding_needed))
    }
}

