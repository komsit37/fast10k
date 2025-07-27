//! Status display component for showing messages and progress

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::edinet_tui::ui::Styles;

/// Types of status messages
#[derive(Debug, Clone, PartialEq)]
pub enum StatusType {
    Info,
    Success,
    Warning,
    Error,
    Loading,
}

/// Status message with type and content
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub message: String,
    pub status_type: StatusType,
    pub timestamp: Option<chrono::DateTime<chrono::Local>>,
}

impl StatusMessage {
    pub fn new(message: String, status_type: StatusType) -> Self {
        Self {
            message,
            status_type,
            timestamp: Some(chrono::Local::now()),
        }
    }

    pub fn info(message: String) -> Self {
        Self::new(message, StatusType::Info)
    }

    pub fn success(message: String) -> Self {
        Self::new(message, StatusType::Success)
    }

    pub fn warning(message: String) -> Self {
        Self::new(message, StatusType::Warning)
    }

    pub fn error(message: String) -> Self {
        Self::new(message, StatusType::Error)
    }

    pub fn loading(message: String) -> Self {
        Self::new(message, StatusType::Loading)
    }
}

/// Status display component
pub struct StatusDisplay {
    pub current_message: Option<StatusMessage>,
    pub message_history: Vec<StatusMessage>,
    pub max_history: usize,
    pub show_timestamp: bool,
    pub auto_clear_timeout: Option<std::time::Duration>,
}

impl Default for StatusDisplay {
    fn default() -> Self {
        Self {
            current_message: None,
            message_history: Vec::new(),
            max_history: 100,
            show_timestamp: false,
            auto_clear_timeout: None,
        }
    }
}

impl StatusDisplay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_history(mut self, max_history: usize) -> Self {
        self.max_history = max_history;
        self
    }

    pub fn with_timestamps(mut self) -> Self {
        self.show_timestamp = true;
        self
    }

    pub fn with_auto_clear(mut self, timeout: std::time::Duration) -> Self {
        self.auto_clear_timeout = Some(timeout);
        self
    }

    /// Set current status message
    pub fn set_message(&mut self, message: StatusMessage) {
        // Add to history if we have a current message
        if let Some(current) = self.current_message.take() {
            self.message_history.push(current);
            
            // Trim history if needed
            if self.message_history.len() > self.max_history {
                self.message_history.remove(0);
            }
        }

        self.current_message = Some(message);
    }

    /// Set info message
    pub fn set_info(&mut self, message: String) {
        self.set_message(StatusMessage::info(message));
    }

    /// Set success message
    pub fn set_success(&mut self, message: String) {
        self.set_message(StatusMessage::success(message));
    }

    /// Set warning message
    pub fn set_warning(&mut self, message: String) {
        self.set_message(StatusMessage::warning(message));
    }

    /// Set error message
    pub fn set_error(&mut self, message: String) {
        self.set_message(StatusMessage::error(message));
    }

    /// Set loading message
    pub fn set_loading(&mut self, message: String) {
        self.set_message(StatusMessage::loading(message));
    }

    /// Clear current message
    pub fn clear(&mut self) {
        if let Some(current) = self.current_message.take() {
            self.message_history.push(current);
            
            // Trim history if needed
            if self.message_history.len() > self.max_history {
                self.message_history.remove(0);
            }
        }
    }

    /// Get current message
    pub fn get_current(&self) -> Option<&StatusMessage> {
        self.current_message.as_ref()
    }

    /// Get message history
    pub fn get_history(&self) -> &[StatusMessage] {
        &self.message_history
    }

    /// Check if we should auto-clear the current message
    pub fn should_auto_clear(&self) -> bool {
        if let (Some(timeout), Some(message)) = (self.auto_clear_timeout, &self.current_message) {
            if let Some(timestamp) = message.timestamp {
                let elapsed = chrono::Local::now().signed_duration_since(timestamp);
                return elapsed.to_std().unwrap_or_default() > timeout;
            }
        }
        false
    }

    /// Render the status display
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let content = if let Some(message) = &self.current_message {
            self.format_message(message)
        } else {
            "Ready".to_string()
        };

        let style = if let Some(message) = &self.current_message {
            match message.status_type {
                StatusType::Info => Styles::info(),
                StatusType::Success => Styles::success(),
                StatusType::Warning => Styles::warning(),
                StatusType::Error => Styles::error(),
                StatusType::Loading => Styles::warning(),
            }
        } else {
            Styles::default()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Styles::inactive_border());

        let paragraph = Paragraph::new(content)
            .style(style)
            .block(block);

        f.render_widget(paragraph, area);
    }

    /// Render with custom title
    pub fn render_with_title(&self, f: &mut Frame, area: Rect, title: &str) {
        let content = if let Some(message) = &self.current_message {
            self.format_message(message)
        } else {
            "Ready".to_string()
        };

        let style = if let Some(message) = &self.current_message {
            match message.status_type {
                StatusType::Info => Styles::info(),
                StatusType::Success => Styles::success(),
                StatusType::Warning => Styles::warning(),
                StatusType::Error => Styles::error(),
                StatusType::Loading => Styles::warning(),
            }
        } else {
            Styles::default()
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Styles::inactive_border());

        let paragraph = Paragraph::new(content)
            .style(style)
            .block(block);

        f.render_widget(paragraph, area);
    }

    /// Format message for display
    fn format_message(&self, message: &StatusMessage) -> String {
        let prefix = match message.status_type {
            StatusType::Info => "ℹ",
            StatusType::Success => "✓",
            StatusType::Warning => "⚠",
            StatusType::Error => "✗",
            StatusType::Loading => "⟳",
        };

        if self.show_timestamp {
            if let Some(timestamp) = message.timestamp {
                format!(
                    "{} [{}] {}",
                    prefix,
                    timestamp.format("%H:%M:%S"),
                    message.message
                )
            } else {
                format!("{} {}", prefix, message.message)
            }
        } else {
            format!("{} {}", prefix, message.message)
        }
    }
}