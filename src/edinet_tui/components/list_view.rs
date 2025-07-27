//! Generic list view component

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::edinet_tui::{ui::Styles, traits::Navigable};

/// Configuration for list view rendering
#[derive(Debug, Clone)]
pub struct ListViewConfig {
    pub title: String,
    pub show_index: bool,
    pub highlight_selected: bool,
    pub show_borders: bool,
    pub max_items: Option<usize>,
}

impl Default for ListViewConfig {
    fn default() -> Self {
        Self {
            title: "List".to_string(),
            show_index: false,
            highlight_selected: true,
            show_borders: true,
            max_items: None,
        }
    }
}

impl ListViewConfig {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            ..Default::default()
        }
    }

    pub fn with_index(mut self) -> Self {
        self.show_index = true;
        self
    }

    pub fn without_borders(mut self) -> Self {
        self.show_borders = false;
        self
    }

    pub fn with_max_items(mut self, max: usize) -> Self {
        self.max_items = Some(max);
        self
    }
}

/// Generic list view component
pub struct ListView<T> {
    pub items: Vec<T>,
    pub state: ListState,
    pub config: ListViewConfig,
}

impl<T> ListView<T> {
    pub fn new(items: Vec<T>, config: ListViewConfig) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        
        Self {
            items,
            state,
            config,
        }
    }

    pub fn with_selection(mut self, selected: Option<usize>) -> Self {
        self.state.select(selected);
        self
    }

    /// Update items and maintain selection if possible
    pub fn set_items(&mut self, items: Vec<T>) {
        let selected = self.state.selected();
        self.items = items;
        
        // Maintain selection if still valid
        if let Some(idx) = selected {
            if idx < self.items.len() {
                self.state.select(Some(idx));
            } else if !self.items.is_empty() {
                self.state.select(Some(0));
            } else {
                self.state.select(None);
            }
        } else if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }

    /// Get currently selected item
    pub fn selected(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    /// Get selected index
    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Select item by index
    pub fn select(&mut self, index: Option<usize>) {
        self.state.select(index);
    }

    /// Navigate to next item
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

    /// Navigate to previous item
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

    /// Render the list view
    pub fn render<F>(&mut self, f: &mut Frame, area: Rect, item_formatter: F)
    where
        F: Fn(usize, &T, bool) -> ListItem,
    {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .take(self.config.max_items.unwrap_or(usize::MAX))
            .map(|(i, item)| {
                let is_selected = Some(i) == self.state.selected();
                item_formatter(i, item, is_selected)
            })
            .collect();

        let block = if self.config.show_borders {
            Block::default()
                .title(self.config.title.clone())
                .borders(Borders::ALL)
                .border_style(Styles::active_border())
        } else {
            Block::default()
        };

        let list = List::new(items)
            .block(block)
            .highlight_style(if self.config.highlight_selected {
                Styles::selected()
            } else {
                Style::default()
            });

        f.render_stateful_widget(list, area, &mut self.state);
    }

    /// Helper to render simple string items
    pub fn render_strings(&mut self, f: &mut Frame, area: Rect)
    where
        T: AsRef<str>,
    {
        let show_index = self.config.show_index;
        let highlight_selected = self.config.highlight_selected;
        
        self.render(f, area, |i, item, is_selected| {
            let content = if show_index {
                format!("{}. {}", i + 1, item.as_ref())
            } else {
                item.as_ref().to_string()
            };

            let style = if is_selected && highlight_selected {
                Styles::selected()
            } else {
                Style::default()
            };

            ListItem::new(Line::from(Span::styled(content, style)))
        });
    }
}

/// Specialized list view for menu items
pub struct MenuListView {
    pub items: Vec<MenuItem>,
    pub list_view: ListView<MenuItem>,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<char>,
    pub description: Option<String>,
    pub enabled: bool,
}

impl MenuItem {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            shortcut: None,
            description: None,
            enabled: true,
        }
    }

    pub fn with_shortcut(mut self, shortcut: char) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

impl MenuListView {
    pub fn new(items: Vec<MenuItem>, title: &str) -> Self {
        let list_view = ListView::new(items.clone(), ListViewConfig::new(title));
        Self { items, list_view }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        self.list_view.render(f, area, |_, item, is_selected| {
            let shortcut_text = if let Some(shortcut) = item.shortcut {
                format!("[{}] ", shortcut)
            } else {
                "    ".to_string()
            };

            let content = format!("{}{}", shortcut_text, item.label);
            
            let style = if !item.enabled {
                Styles::inactive()
            } else if is_selected {
                Styles::selected()
            } else {
                Style::default()
            };

            ListItem::new(Line::from(Span::styled(content, style)))
        });
    }

    /// Get currently selected menu item
    pub fn selected(&self) -> Option<&MenuItem> {
        self.list_view.selected()
    }

    /// Navigate to next item
    pub fn next(&mut self) {
        self.list_view.next();
    }

    /// Navigate to previous item
    pub fn previous(&mut self) {
        self.list_view.previous();
    }

    /// Select by shortcut key
    pub fn select_by_shortcut(&mut self, key: char) -> bool {
        for (i, item) in self.items.iter().enumerate() {
            if let Some(shortcut) = item.shortcut {
                if shortcut.to_ascii_uppercase() == key.to_ascii_uppercase() {
                    self.list_view.select(Some(i));
                    return true;
                }
            }
        }
        false
    }
}

impl Navigable for MenuListView {
    fn navigate_up(&mut self) {
        self.previous();
    }

    fn navigate_down(&mut self) {
        self.next();
    }

    fn get_selected_index(&self) -> Option<usize> {
        self.list_view.selected_index()
    }

    fn set_selected_index(&mut self, index: Option<usize>) {
        self.list_view.select(index);
    }

    fn get_item_count(&self) -> usize {
        self.items.len()
    }
}