//! Common UI components and utilities for the EDINET TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

/// Common UI styles
pub struct Styles;

impl Styles {
    pub fn default() -> Style {
        Style::default()
    }

    pub fn selected() -> Style {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    pub fn title() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    pub fn error() -> Style {
        Style::default()
            .fg(Color::Red)
    }

    pub fn success() -> Style {
        Style::default()
            .fg(Color::Green)
    }

    pub fn warning() -> Style {
        Style::default()
            .fg(Color::Yellow)
    }

    pub fn info() -> Style {
        Style::default()
            .fg(Color::Cyan)
    }

    pub fn inactive() -> Style {
        Style::default()
            .fg(Color::Gray)
    }

    pub fn active_border() -> Style {
        Style::default()
            .fg(Color::Yellow)
    }

    pub fn inactive_border() -> Style {
        Style::default()
            .fg(Color::Gray)
    }
}

/// Selectable list widget with state
pub struct SelectableList<T> {
    pub items: Vec<T>,
    pub state: ListState,
}

impl<T> SelectableList<T> {
    pub fn new(items: Vec<T>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self { items, state }
    }

    pub fn with_items(items: Vec<T>) -> Self {
        Self::new(items)
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => (i + 1) % self.items.len(),
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn selected(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.state.select(index);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Input field widget
#[derive(Clone)]
pub struct InputField {
    pub label: String,
    pub value: String,
    pub placeholder: String,
    pub is_focused: bool,
    pub cursor_position: usize,
}

impl InputField {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            value: String::new(),
            placeholder: String::new(),
            is_focused: false,
            cursor_position: 0,
        }
    }

    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.to_string();
        self
    }

    pub fn with_value(mut self, value: &str) -> Self {
        self.value = value.to_string();
        self.cursor_position = value.len();
        self
    }

    pub fn set_focus(&mut self, focused: bool) {
        self.is_focused = focused;
    }

    pub fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.value.remove(self.cursor_position);
        }
    }

    pub fn delete_char_forward(&mut self) {
        if self.cursor_position < self.value.len() {
            self.value.remove(self.cursor_position);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.value.len() {
            self.cursor_position += 1;
        }
    }

    pub fn move_cursor_to_start(&mut self) {
        self.cursor_position = 0;
    }

    pub fn move_cursor_to_end(&mut self) {
        self.cursor_position = self.value.len();
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Render the input field as a widget
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let display_text = if self.value.is_empty() && !self.placeholder.is_empty() {
            &self.placeholder
        } else {
            &self.value
        };

        let style = if self.is_focused {
            Styles::active_border()
        } else {
            Styles::inactive_border()
        };

        let block = Block::default()
            .title(self.label.as_str())
            .borders(Borders::ALL)
            .border_style(style);

        let input_style = if self.value.is_empty() && !self.placeholder.is_empty() {
            Styles::inactive()
        } else {
            Styles::default()
        };

        let paragraph = Paragraph::new(display_text.to_string())
            .style(input_style)
            .block(block);

        f.render_widget(paragraph, area);

        // Render cursor if focused
        if self.is_focused {
            let cursor_x = area.x + 1 + self.cursor_position as u16;
            let cursor_y = area.y + 1;
            if cursor_x < area.x + area.width - 1 {
                f.set_cursor(cursor_x, cursor_y);
            }
        }
    }
}

/// Table-like display for documents
pub fn render_document_table(
    f: &mut Frame,
    area: Rect,
    documents: &[crate::models::Document],
    selected_index: Option<usize>,
    title: &str,
) {
    let items: Vec<ListItem> = documents
        .iter()
        .enumerate()
        .map(|(i, doc)| {
            let style = if Some(i) == selected_index {
                Styles::selected()
            } else {
                Style::default()
            };

            let content = format!(
                "{} | {} | {} | {} | {}",
                doc.date,
                doc.ticker.get(0..8).unwrap_or(&doc.ticker),
                doc.company_name.get(0..20).unwrap_or(&doc.company_name),
                doc.filing_type.as_str().get(0..8).unwrap_or(doc.filing_type.as_str()),
                doc.format.as_str()
            );

            ListItem::new(Line::from(Span::styled(content, style)))
        })
        .collect();

    let header = Line::from(vec![
        Span::styled("Date      ", Styles::title()),
        Span::styled("| Symbol   ", Styles::title()),
        Span::styled("| Company             ", Styles::title()),
        Span::styled("| Type     ", Styles::title()),
        Span::styled("| Format", Styles::title()),
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Styles::active_border());

    // Create list with header
    let mut list_items = vec![ListItem::new(header)];
    list_items.extend(items);

    let list = List::new(list_items).block(block);

    f.render_widget(list, area);
}

/// Center a rectangle within another rectangle
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Create a popup area
pub fn popup_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    centered_rect(percent_x, percent_y, r)
}

/// Text wrapping utility
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.len() <= width {
            lines.push(line.to_string());
        } else {
            let mut current_line = String::new();
            for word in line.split_whitespace() {
                if current_line.len() + word.len() + 1 <= width {
                    if !current_line.is_empty() {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                } else {
                    if !current_line.is_empty() {
                        lines.push(current_line);
                        current_line = String::new();
                    }
                    if word.len() > width {
                        // Split long words
                        let mut start = 0;
                        while start < word.len() {
                            let end = std::cmp::min(start + width, word.len());
                            lines.push(word[start..end].to_string());
                            start = end;
                        }
                    } else {
                        current_line = word.to_string();
                    }
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }
    }
    lines
}