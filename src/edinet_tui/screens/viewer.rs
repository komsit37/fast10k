//! Document viewer screen for the EDINET TUI

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::path::PathBuf;

use crate::{
    models::{Document, DownloadRequest, DocumentFormat, Source},
    downloader,
    edinet::reader::{read_edinet_zip, DocumentSection},
    edinet_tui::ui::Styles,
};

/// Document viewer mode
#[derive(Debug, Clone, PartialEq)]
pub enum ViewerMode {
    Info,      // Document metadata
    Content,   // Document content sections
    Download,  // Download options
}

/// Document viewer screen state
pub struct ViewerScreen {
    pub current_document: Option<Document>,
    pub mode: ViewerMode,
    pub scroll_offset: usize,
    pub content_sections: Option<Vec<DocumentSection>>,
    pub current_section: usize,
    pub is_loading: bool,
    pub is_downloading: bool,
    pub download_status: Option<String>,
    pub is_downloaded: bool,
    pub pending_g_key: bool, // For "gg" command
}

impl ViewerScreen {
    pub fn new() -> Self {
        Self {
            current_document: None,
            mode: ViewerMode::Info,
            scroll_offset: 0,
            content_sections: None,
            current_section: 0,
            is_loading: false,
            is_downloading: false,
            download_status: None,
            is_downloaded: false,
            pending_g_key: false,
        }
    }

    /// Set document to view
    pub fn set_document(&mut self, document: Document) {
        self.current_document = Some(document);
        self.mode = ViewerMode::Info;
        self.scroll_offset = 0;
        self.content_sections = None;
        self.current_section = 0;
        self.is_loading = false;
        self.is_downloaded = false; // Will be updated when checked
    }

    /// Handle key events for the viewer screen
    pub async fn handle_event(&mut self, key: KeyEvent, app: &mut super::super::app::App) -> Result<()> {
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
            KeyCode::Tab => {
                // Switch between modes
                self.mode = match self.mode {
                    ViewerMode::Info => ViewerMode::Content,
                    ViewerMode::Content => ViewerMode::Download,
                    ViewerMode::Download => ViewerMode::Info,
                };
                self.scroll_offset = 0;
            }
            KeyCode::Up => {
                match self.mode {
                    ViewerMode::Info | ViewerMode::Download => {
                        if self.scroll_offset > 0 {
                            self.scroll_offset -= 1;
                        }
                    }
                    ViewerMode::Content => {
                        if self.content_sections.is_some() && self.current_section > 0 {
                            self.current_section -= 1;
                            self.scroll_offset = 0;
                        }
                    }
                }
            }
            KeyCode::Down => {
                match self.mode {
                    ViewerMode::Info | ViewerMode::Download => {
                        self.scroll_offset += 1;
                    }
                    ViewerMode::Content => {
                        if let Some(ref sections) = self.content_sections {
                            if self.current_section < sections.len() - 1 {
                                self.current_section += 1;
                                self.scroll_offset = 0;
                            }
                        }
                    }
                }
            }
            KeyCode::PageUp => {
                match self.mode {
                    ViewerMode::Info | ViewerMode::Download => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(10);
                    }
                    ViewerMode::Content => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(10);
                    }
                }
            }
            KeyCode::PageDown => {
                match self.mode {
                    ViewerMode::Info | ViewerMode::Download => {
                        self.scroll_offset += 10;
                    }
                    ViewerMode::Content => {
                        self.scroll_offset += 10;
                    }
                }
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
                if self.mode == ViewerMode::Content {
                    self.current_section = 0;
                }
            }
            KeyCode::End => {
                if self.mode == ViewerMode::Content {
                    if let Some(ref sections) = self.content_sections {
                        self.current_section = sections.len().saturating_sub(1);
                    }
                }
                self.scroll_offset = 0;
            }
            KeyCode::Enter => {
                match self.mode {
                    ViewerMode::Content => {
                        // Load content if not already loaded
                        self.load_document_content(app).await?;
                    }
                    ViewerMode::Download => {
                        // Download document
                        self.download_document(app).await?;
                    }
                    ViewerMode::Info => {
                        // Switch to content view
                        self.mode = ViewerMode::Content;
                        self.load_document_content(app).await?;
                    }
                }
            }
            KeyCode::Char('d') => {
                // Download document
                self.download_document(app).await?;
            }
            KeyCode::Char('r') => {
                // Reload/refresh content
                if self.mode == ViewerMode::Content {
                    self.content_sections = None;
                    self.load_document_content(app).await?;
                }
            }
            KeyCode::Char('s') => {
                // Save content to file (placeholder)
                app.set_status("Save functionality not implemented yet".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    /// Load document content from downloaded ZIP file
    async fn load_document_content(&mut self, app: &mut super::super::app::App) -> Result<()> {
        if self.content_sections.is_some() {
            return Ok(()); // Already loaded
        }

        let document = match &self.current_document {
            Some(doc) => doc,
            None => return Ok(()),
        };

        self.is_loading = true;
        app.set_status("Loading document content...".to_string());

        // Construct expected download path
        let download_dir = PathBuf::from(app.config.download_dir_str());
        let edinet_dir = download_dir.join("edinet").join(&document.ticker);

        // Look for ZIP files in the directory
        if let Ok(entries) = std::fs::read_dir(&edinet_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("zip") {
                    match read_edinet_zip(path.to_str().unwrap(), usize::MAX, usize::MAX) {
                        Ok(sections) => {
                            self.content_sections = Some(sections);
                            self.current_section = 0;
                            self.is_loading = false;
                            app.set_status("Document content loaded".to_string());
                            return Ok(());
                        }
                        Err(e) => {
                            app.set_error(format!("Failed to read document: {}", e));
                            self.is_loading = false;
                            return Ok(());
                        }
                    }
                }
            }
        }

        // If no downloaded file found, suggest downloading
        app.set_error("Document not found locally. Use 'd' to download first.".to_string());
        self.is_loading = false;
        Ok(())
    }

    /// Check if document is downloaded
    pub fn is_document_downloaded(&self, app: &super::super::app::App) -> bool {
        let document = match &self.current_document {
            Some(doc) => doc,
            None => return false,
        };

        // Get the document ID from metadata for precise matching
        let doc_id = document.metadata.get("doc_id")
            .or_else(|| document.metadata.get("document_id"))
            .unwrap_or(&document.id);

        // Check if the specific ZIP file exists in download directory
        let download_dir = std::path::PathBuf::from(app.config.download_dir_str())
            .join("edinet")
            .join(&document.ticker);
        
        if let Ok(entries) = std::fs::read_dir(&download_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("zip") {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        // Check if this ZIP file matches our document ID
                        if filename.contains(doc_id) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Download document
    async fn download_document(&mut self, app: &mut super::super::app::App) -> Result<()> {
        let document = match &self.current_document {
            Some(doc) => doc.clone(),
            None => return Ok(()),
        };

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

        match downloader::download_documents(&download_request, app.config.download_dir_str()).await {
            Ok(count) => {
                app.set_status(format!("Successfully downloaded {} document(s)", count));
                // Clear content sections to force reload
                self.content_sections = None;
                // Update download status
                self.is_downloaded = self.is_document_downloaded(app);
            }
            Err(e) => {
                app.set_error(format!("Download failed: {}", e));
            }
        }

        self.is_downloading = false;
        self.download_status = None;
        Ok(())
    }

    /// Draw the viewer screen
    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        if self.current_document.is_none() {
            self.draw_no_document(f, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(0),     // Content
                Constraint::Length(3),  // Mode selector/instructions
            ])
            .split(area);

        // Draw title
        self.draw_title(f, chunks[0]);
        
        // Draw content based on mode
        match self.mode {
            ViewerMode::Info => self.draw_info_mode(f, chunks[1]),
            ViewerMode::Content => self.draw_content_mode(f, chunks[1]),
            ViewerMode::Download => self.draw_download_mode(f, chunks[1]),
        }
        
        // Draw mode selector and instructions
        self.draw_bottom_bar(f, chunks[2]);

        // Draw download status if downloading
        if self.is_downloading {
            self.draw_download_status(f, area);
        }
    }

    fn draw_no_document(&self, f: &mut Frame, area: Rect) {
        let message = Paragraph::new("No document selected\n\nPress ESC to go back")
            .style(Styles::inactive())
            .block(Block::default()
                .title("Document Viewer")
                .borders(Borders::ALL)
                .border_style(Styles::inactive_border()));
        f.render_widget(message, area);
    }

    fn draw_title(&self, f: &mut Frame, area: Rect) {
        let document = self.current_document.as_ref().unwrap();
        let title_text = format!("{} - {} ({})", 
            document.ticker, 
            document.company_name, 
            document.date
        );

        let title = Paragraph::new(title_text)
            .style(Styles::title())
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn draw_info_mode(&self, f: &mut Frame, area: Rect) {
        let document = self.current_document.as_ref().unwrap();
        
        let info_lines = vec![
            Line::from(vec![
                Span::styled("Ticker: ", Styles::info()),
                Span::raw(&document.ticker),
            ]),
            Line::from(vec![
                Span::styled("Company: ", Styles::info()),
                Span::raw(&document.company_name),
            ]),
            Line::from(vec![
                Span::styled("Filing Type: ", Styles::info()),
                Span::raw(document.filing_type.as_str()),
            ]),
            Line::from(vec![
                Span::styled("Date: ", Styles::info()),
                Span::raw(document.date.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Source: ", Styles::info()),
                Span::raw(document.source.as_str()),
            ]),
            Line::from(vec![
                Span::styled("Format: ", Styles::info()),
                Span::raw(document.format.as_str()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Content Path: ", Styles::info()),
                Span::raw(document.content_path.to_string_lossy()),
            ]),
        ];

        // Add metadata if available
        let mut all_lines = info_lines;
        if !document.metadata.is_empty() {
            all_lines.push(Line::from(""));
            all_lines.push(Line::from(Span::styled("Metadata:", Styles::info())));
            for (key, value) in &document.metadata {
                all_lines.push(Line::from(format!("  {}: {}", key, value)));
            }
        }

        // Add download status and file information
        all_lines.push(Line::from(""));
        self.add_download_info(&mut all_lines, document);

        // Apply scrolling
        let visible_lines: Vec<Line> = all_lines
            .into_iter()
            .skip(self.scroll_offset)
            .collect();

        let info_widget = Paragraph::new(visible_lines)
            .block(Block::default()
                .title("Document Information")
                .borders(Borders::ALL)
                .border_style(Styles::active_border()))
            .wrap(Wrap { trim: true });

        f.render_widget(info_widget, area);
    }

    fn draw_content_mode(&self, f: &mut Frame, area: Rect) {
        if let Some(ref sections) = self.content_sections {
            if sections.is_empty() {
                let empty_widget = Paragraph::new("No content sections found")
                    .style(Styles::inactive())
                    .block(Block::default()
                        .title("Document Content")
                        .borders(Borders::ALL)
                        .border_style(Styles::active_border()));
                f.render_widget(empty_widget, area);
                return;
            }

            let current_section = &sections[self.current_section];
            
            let content_lines = vec![
                Line::from(vec![
                    Span::styled("Section: ", Styles::info()),
                    Span::styled(&current_section.section_type, Styles::title()),
                ]),
                Line::from(vec![
                    Span::styled("File: ", Styles::info()),
                    Span::raw(&current_section.filename),
                ]),
                Line::from(vec![
                    Span::styled("Size: ", Styles::info()),
                    Span::raw(format!("{} characters", current_section.full_length)),
                ]),
                Line::from(""),
            ];

            // Add content lines
            let mut all_lines = content_lines;
            for line in current_section.content.lines() {
                all_lines.push(Line::from(Span::raw(line)));
            }

            // Apply scrolling
            let visible_lines: Vec<Line> = all_lines
                .into_iter()
                .skip(self.scroll_offset)
                .collect();

            let title = format!("Content ({}/{})", 
                self.current_section + 1, 
                sections.len()
            );

            let content_widget = Paragraph::new(visible_lines)
                .block(Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Styles::active_border()))
                .wrap(Wrap { trim: true });

            f.render_widget(content_widget, area);
        } else if self.is_loading {
            let loading_widget = Paragraph::new("Loading content...")
                .style(Styles::info())
                .block(Block::default()
                    .title("Document Content")
                    .borders(Borders::ALL)
                    .border_style(Styles::active_border()));
            f.render_widget(loading_widget, area);
        } else {
            // Check if document is downloaded and provide appropriate message
            let message = if self.is_downloaded {
                "Press Enter to load content"
            } else {
                "Press Enter to load content\n\nNote: Document must be downloaded first"
            };

            let message_widget = Paragraph::new(message)
                .style(Styles::inactive())
                .block(Block::default()
                    .title("Document Content")
                    .borders(Borders::ALL)
                    .border_style(Styles::active_border()));
            f.render_widget(message_widget, area);
        }
    }

    fn draw_download_mode(&self, f: &mut Frame, area: Rect) {
        let document = self.current_document.as_ref().unwrap();
        
        let download_info = vec![
            Line::from(vec![
                Span::styled("Document: ", Styles::info()),
                Span::raw(format!("{} - {}", document.ticker, document.company_name)),
            ]),
            Line::from(vec![
                Span::styled("Type: ", Styles::info()),
                Span::raw(document.filing_type.as_str()),
            ]),
            Line::from(vec![
                Span::styled("Date: ", Styles::info()),
                Span::raw(document.date.to_string()),
            ]),
            Line::from(""),
            Line::from("Download Options:"),
            Line::from(""),
            Line::from("• Press Enter or 'd' to download complete document"),
            Line::from("• Files will be saved to the downloads directory"),
            Line::from("• EDINET documents are downloaded as ZIP files"),
            Line::from("• Content can be viewed after download"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Styles::info()),
                Span::raw(if self.is_downloaded {
                    "Document downloaded"
                } else {
                    "Document not downloaded"
                }),
            ]),
        ];

        let download_widget = Paragraph::new(download_info)
            .block(Block::default()
                .title("Download")
                .borders(Borders::ALL)
                .border_style(Styles::active_border()))
            .wrap(Wrap { trim: true });

        f.render_widget(download_widget, area);
    }

    fn draw_bottom_bar(&self, f: &mut Frame, area: Rect) {
        let mode_indicator = match self.mode {
            ViewerMode::Info => "[Info]",
            ViewerMode::Content => "[Content]",
            ViewerMode::Download => "[Download]",
        };

        let instructions = match self.mode {
            ViewerMode::Info => "Tab: Switch mode | ↑/↓: Scroll | Enter: View content",
            ViewerMode::Content => "Tab: Switch mode | ↑/↓: Sections | PgUp/PgDn: Scroll | r: Reload",
            ViewerMode::Download => "Tab: Switch mode | Enter/d: Download | s: Save",
        };

        let bottom_text = format!("{} | {} | ESC: Back", mode_indicator, instructions);

        let bottom_widget = Paragraph::new(bottom_text)
            .style(Styles::info())
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(bottom_widget, area);
    }

    fn draw_download_status(&self, f: &mut Frame, area: Rect) {
        use crate::edinet_tui::ui::centered_rect;
        
        let popup_area = centered_rect(50, 20, area);
        
        let default_status = "Downloading...".to_string();
        let status_text = self.download_status
            .as_ref()
            .unwrap_or(&default_status);
        
        let status_widget = Paragraph::new(format!("{}\n\nPress ESC to cancel", status_text))
            .style(Styles::info())
            .block(Block::default()
                .title("Download Status")
                .borders(Borders::ALL)
                .border_style(Styles::warning()));

        f.render_widget(ratatui::widgets::Clear, popup_area);
        f.render_widget(status_widget, popup_area);
    }

    /// Add download status and file information to the info display
    fn add_download_info(&self, lines: &mut Vec<Line>, document: &Document) {
        // Get the document ID from metadata for precise matching
        let doc_id = document.metadata.get("doc_id")
            .or_else(|| document.metadata.get("document_id"))
            .unwrap_or(&document.id);

        // Check download status and get file path - using default download path
        // This should ideally use the config, but for now we'll use the default
        let download_dir = std::path::PathBuf::from("./downloads")
            .join("edinet")
            .join(&document.ticker);

        let mut downloaded_file_path = None;
        let mut zip_contents = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&download_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("zip") {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.contains(doc_id) {
                            downloaded_file_path = Some(path.clone());
                            // Try to read ZIP contents
                            if let Ok(contents) = self.read_zip_contents(&path) {
                                zip_contents = contents;
                            }
                            break;
                        }
                    }
                }
            }
        }

        // Add download status
        if let Some(file_path) = downloaded_file_path {
            lines.push(Line::from(vec![
                Span::styled("Download Status: ", Styles::info()),
                Span::styled("Downloaded", Styles::success()),
            ]));
            
            if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                lines.push(Line::from(vec![
                    Span::styled("File Name: ", Styles::info()),
                    Span::raw(filename.to_string()),
                ]));
            }

            if let Ok(metadata) = std::fs::metadata(&file_path) {
                let file_size = if metadata.len() < 1024 * 1024 {
                    format!("{:.1} KB", metadata.len() as f64 / 1024.0)
                } else {
                    format!("{:.1} MB", metadata.len() as f64 / (1024.0 * 1024.0))
                };
                lines.push(Line::from(vec![
                    Span::styled("File Size: ", Styles::info()),
                    Span::raw(file_size),
                ]));
            }

            // Add ZIP contents if available
            if !zip_contents.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("ZIP Contents:", Styles::info())));
                for (filename, size) in zip_contents {
                    let size_str = if size < 1024 {
                        format!("{} B", size)
                    } else if size < 1024 * 1024 {
                        format!("{:.1} KB", size as f64 / 1024.0)
                    } else {
                        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                    };
                    lines.push(Line::from(format!("  {} ({})", filename, size_str)));
                }
            }
        } else {
            lines.push(Line::from(vec![
                Span::styled("Download Status: ", Styles::info()),
                Span::styled("Not Downloaded", Styles::error()),
            ]));
            lines.push(Line::from("  Use 'd' to download or Tab to Download mode"));
        }
    }

    /// Read ZIP file contents and return list of files with sizes
    fn read_zip_contents(&self, zip_path: &std::path::Path) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
        use std::fs::File;
        use zip::ZipArchive;

        let file = File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut contents = Vec::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            contents.push((file.name().to_string(), file.size()));
        }

        // Sort by filename for consistent display
        contents.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(contents)
    }
}