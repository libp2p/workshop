use crate::ui::tui::widgets::ScrollText;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, StatefulWidget, Widget},
};

#[derive(Clone, Debug, Default)]
pub struct ScrollBox<'a> {
    /// the scrollable text widget
    st: ScrollText<'a>,
    /// the text to render
    text: String,
}

impl<'a> ScrollBox<'a> {
    pub fn set_text<S: AsRef<str>>(&mut self, text: S) {
        self.text = text.as_ref().to_string();
    }

    pub fn style(&mut self, style: Style) {
        self.st.style(style);
    }

    pub fn block(&mut self, block: Block<'a>) {
        self.st.block(block);
    }

    /// Scroll to the top
    pub fn scroll_top(&mut self) {
        self.st.scroll_top();
    }

    /// Scroll to the bottom
    pub fn scroll_bottom(&mut self) {
        self.st.scroll_bottom();
    }

    /// Scroll up
    pub fn scroll_up(&mut self) {
        self.st.scroll_up();
    }

    /// Scroll down
    pub fn scroll_down(&mut self) {
        self.st.scroll_down();
    }
}

impl Widget for &mut ScrollBox<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // render the scroll text
        StatefulWidget::render(&mut self.st, area, buf, &mut self.text);
    }
}
