//! Form field component for user input

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::edinet_tui::{traits::FormHandler, ui::Styles};

/// Type of form field
#[derive(Debug, Clone, PartialEq)]
pub enum FormFieldType {
    Text,
    Date,
    Dropdown,
    TextArea,
}

/// Individual form field
#[derive(Debug, Clone)]
pub struct FormField {
    pub label: String,
    pub value: String,
    pub placeholder: String,
    pub field_type: FormFieldType,
    pub is_focused: bool,
    pub cursor_position: usize,
    pub dropdown_options: Vec<String>,
    pub dropdown_state: ListState,
    pub show_dropdown: bool,
    pub validation_error: Option<String>,
}

impl FormField {
    pub fn new(label: &str, field_type: FormFieldType) -> Self {
        Self {
            label: label.to_string(),
            value: String::new(),
            placeholder: String::new(),
            field_type,
            is_focused: false,
            cursor_position: 0,
            dropdown_options: Vec::new(),
            dropdown_state: ListState::default(),
            show_dropdown: false,
            validation_error: None,
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

    pub fn with_dropdown_options(mut self, options: Vec<String>) -> Self {
        self.dropdown_options = options;
        if !self.dropdown_options.is_empty() {
            self.dropdown_state.select(Some(0));
        }
        self
    }

    pub fn set_focus(&mut self, focused: bool) {
        self.is_focused = focused;
        if focused && self.field_type == FormFieldType::Dropdown {
            self.show_dropdown = true;
        } else if !focused {
            self.show_dropdown = false;
        }
    }

    pub fn insert_char(&mut self, c: char) {
        if self.field_type == FormFieldType::Dropdown {
            return; // Don't allow typing in dropdown
        }
        self.value.insert(self.cursor_position, c);
        self.cursor_position += 1;
        self.validation_error = None;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.value.remove(self.cursor_position);
            self.validation_error = None;
        }
    }

    pub fn delete_char_forward(&mut self) {
        if self.cursor_position < self.value.len() {
            self.value.remove(self.cursor_position);
            self.validation_error = None;
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
        self.validation_error = None;
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Handle dropdown navigation
    pub fn dropdown_up(&mut self) {
        if self.dropdown_options.is_empty() {
            return;
        }
        let selected = self.dropdown_state.selected().unwrap_or(0);
        let new_selected = if selected == 0 {
            self.dropdown_options.len() - 1
        } else {
            selected - 1
        };
        self.dropdown_state.select(Some(new_selected));
    }

    pub fn dropdown_down(&mut self) {
        if self.dropdown_options.is_empty() {
            return;
        }
        let selected = self.dropdown_state.selected().unwrap_or(0);
        let new_selected = (selected + 1) % self.dropdown_options.len();
        self.dropdown_state.select(Some(new_selected));
    }

    pub fn select_dropdown_value(&mut self) {
        if let Some(selected) = self.dropdown_state.selected() {
            if let Some(value) = self.dropdown_options.get(selected) {
                self.value = value.clone();
                self.cursor_position = self.value.len();
                self.show_dropdown = false;
                self.validation_error = None;
            }
        }
    }

    /// Render the form field
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let display_text = if self.value.is_empty() && !self.placeholder.is_empty() {
            &self.placeholder
        } else {
            &self.value
        };

        let border_style = if self.is_focused {
            Styles::active_border()
        } else if self.validation_error.is_some() {
            Styles::error()
        } else {
            Styles::inactive_border()
        };

        let title = if let Some(ref error) = self.validation_error {
            format!("{} - Error: {}", self.label, error)
        } else {
            self.label.clone()
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let text_style = if self.value.is_empty() && !self.placeholder.is_empty() {
            Styles::inactive()
        } else {
            Styles::default()
        };

        let paragraph = Paragraph::new(display_text.to_string())
            .style(text_style)
            .block(block);

        f.render_widget(paragraph, area);

        // Render cursor if focused and not a dropdown
        if self.is_focused && self.field_type != FormFieldType::Dropdown {
            let cursor_x = area.x + 1 + self.cursor_position as u16;
            let cursor_y = area.y + 1;
            if cursor_x < area.x + area.width - 1 {
                f.set_cursor(cursor_x, cursor_y);
            }
        }
    }

    /// Render dropdown if visible
    pub fn render_dropdown(&mut self, f: &mut Frame, area: Rect) {
        if !self.show_dropdown || self.dropdown_options.is_empty() {
            return;
        }

        let items: Vec<ListItem> = self
            .dropdown_options
            .iter()
            .enumerate()
            .map(|(i, option)| {
                let style = if Some(i) == self.dropdown_state.selected() {
                    Styles::selected()
                } else {
                    Style::default()
                };
                ListItem::new(option.clone()).style(style)
            })
            .collect();

        let block = Block::default()
            .title("Options")
            .borders(Borders::ALL)
            .border_style(Styles::active_border());

        let list = List::new(items).block(block);

        f.render_stateful_widget(list, area, &mut self.dropdown_state);
    }

    /// Validate field value
    pub fn validate(&mut self) -> bool {
        self.validation_error = None;

        match self.field_type {
            FormFieldType::Date => {
                if !self.value.is_empty() {
                    if let Err(_) = chrono::NaiveDate::parse_from_str(&self.value, "%Y-%m-%d") {
                        self.validation_error = Some("Invalid date format (YYYY-MM-DD)".to_string());
                        return false;
                    }
                }
            }
            _ => {}
        }

        true
    }
}

/// Form container that manages multiple fields
pub struct Form {
    pub fields: Vec<FormField>,
    pub current_field: usize,
}

impl Form {
    pub fn new(fields: Vec<FormField>) -> Self {
        let mut form = Self {
            fields,
            current_field: 0,
        };
        form.update_focus();
        form
    }

    fn update_focus(&mut self) {
        for (i, field) in self.fields.iter_mut().enumerate() {
            field.set_focus(i == self.current_field);
        }
    }

    pub fn get_field(&self, index: usize) -> Option<&FormField> {
        self.fields.get(index)
    }

    pub fn get_field_mut(&mut self, index: usize) -> Option<&mut FormField> {
        self.fields.get_mut(index)
    }

    pub fn get_current_field(&self) -> Option<&FormField> {
        self.fields.get(self.current_field)
    }

    pub fn get_current_field_mut(&mut self) -> Option<&mut FormField> {
        self.fields.get_mut(self.current_field)
    }

    /// Validate all fields
    pub fn validate_all(&mut self) -> bool {
        let mut all_valid = true;
        for field in &mut self.fields {
            if !field.validate() {
                all_valid = false;
            }
        }
        all_valid
    }
}

impl FormHandler for Form {
    fn get_current_field(&self) -> usize {
        self.current_field
    }

    fn set_current_field(&mut self, field: usize) {
        if field < self.fields.len() {
            self.current_field = field;
            self.update_focus();
        }
    }

    fn get_field_count(&self) -> usize {
        self.fields.len()
    }

    fn handle_char_input(&mut self, c: char) {
        if let Some(field) = self.get_current_field_mut() {
            field.insert_char(c);
        }
    }

    fn handle_backspace(&mut self) {
        if let Some(field) = self.get_current_field_mut() {
            field.delete_char();
        }
    }

    fn handle_delete(&mut self) {
        if let Some(field) = self.get_current_field_mut() {
            field.delete_char_forward();
        }
    }

    fn validate(&self) -> Result<(), String> {
        for field in &self.fields {
            if let Some(ref error) = field.validation_error {
                return Err(error.clone());
            }
        }
        Ok(())
    }

    async fn submit(&mut self) -> Result<crate::edinet_tui::traits::ScreenAction, anyhow::Error> {
        if self.validate_all() {
            Ok(crate::edinet_tui::traits::ScreenAction::None)
        } else {
            Err(anyhow::anyhow!("Form validation failed"))
        }
    }
}