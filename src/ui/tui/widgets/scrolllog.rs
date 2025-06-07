use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::Line,
    widgets::{
        Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
    },
};
use std::{collections::VecDeque, fmt};
use textwrap;

#[derive(Clone, Debug, Default)]
pub enum Scroll {
    #[default]
    Top,
    MaybeTop(usize),
    Offset(usize),
    MaybeBottom(usize),
    Bottom,
}

impl fmt::Display for Scroll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scroll::Top => write!(f, "Top"),
            Scroll::MaybeTop(offset) => write!(f, "MaybeTop({})", offset),
            Scroll::Offset(offset) => write!(f, "Offset({})", offset),
            Scroll::MaybeBottom(offset) => write!(f, "MaybeBottom({})", offset),
            Scroll::Bottom => write!(f, "Bottom"),
        }
    }
}

/// A vertically scrolling log widget with two columns: emoji and message
#[derive(Clone, Debug, Default)]
pub struct ScrollLog<'a> {
    /// The number of lines of text after wrapping
    lines: usize,
    /// window lines
    window_lines: usize,
    /// The current scroll position
    scroll: Scroll,
    /// The optional surrounding block
    block: Option<Block<'a>>,
    /// The style of the text
    style: Style,
}

impl<'a> ScrollLog<'a> {
    /// add a block
    pub fn block(&mut self, block: Block<'a>) {
        self.block = Some(block);
    }

    /// set the style
    pub fn style(&mut self, style: Style) {
        self.style = style;
    }

    /// get the current scroll position
    pub fn get_scroll(&self) -> &Scroll {
        &self.scroll
    }

    /// get the total lines
    pub fn get_lines(&self) -> usize {
        self.lines
    }

    /// get the window lines
    pub fn get_window_lines(&self) -> usize {
        self.window_lines
    }

    /// Scroll to the top
    pub fn scroll_top(&mut self) {
        self.scroll = Scroll::Top;
    }

    /// Scroll to the bottom
    pub fn scroll_bottom(&mut self) {
        self.scroll = Scroll::Bottom;
    }

    /// Scroll up
    pub fn scroll_up(&mut self) {
        match self.scroll {
            Scroll::Offset(offset) => {
                self.scroll = Scroll::MaybeTop(offset.saturating_sub(1));
            }
            Scroll::Bottom => {
                self.scroll = Scroll::MaybeTop(self.lines.saturating_sub(self.window_lines + 1));
            }
            _ => {}
        }
    }

    /// Scroll down
    pub fn scroll_down(&mut self) {
        match self.scroll {
            Scroll::Top => {
                self.scroll = Scroll::MaybeBottom(1);
            }
            Scroll::Offset(offset) => {
                self.scroll = Scroll::MaybeBottom(offset.saturating_add(1));
            }
            _ => {}
        }
    }
}

impl StatefulWidget for &mut ScrollLog<'_> {
    type State = VecDeque<(Option<String>, String)>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // get the available width after considering block
        let inner_area = if let Some(block) = &self.block {
            block.inner(area)
        } else {
            area
        };

        // left column is 3 characters wide, right column takes the rest
        let left_column_width = 3;
        let right_column_width = inner_area.width.saturating_sub(left_column_width) as usize;

        // collect all log entries and wrap the messages
        let mut all_lines = Vec::new();

        for (emoji, message) in state.iter() {
            let wrap_options = textwrap::Options::new(right_column_width).break_words(true);
            let wrapped_lines = textwrap::wrap(message, &wrap_options);

            // first line includes the emoji
            if let Some(first_line) = wrapped_lines.first() {
                if let Some(emoji_str) = emoji {
                    all_lines.push(format!("{:<2}{}", emoji_str, first_line));
                } else {
                    all_lines.push(format!("{:<3}{}", "", first_line));
                }
            }

            // subsequent lines have blank emoji column
            for line in wrapped_lines.iter().skip(1) {
                all_lines.push(format!("{:<3}{}", "   ", line));
            }
        }

        // get the lines of text after wrapping
        self.lines = all_lines.len();
        // get the lines of the render area
        self.window_lines = inner_area.height as usize;

        // figure out the scroll offset
        let scroll_offset = match self.scroll {
            Scroll::Top => 0,
            Scroll::MaybeTop(offset) => {
                if offset > 0 {
                    self.scroll = Scroll::Offset(offset);
                    offset
                } else {
                    self.scroll = Scroll::Top;
                    0
                }
            }
            Scroll::Offset(offset) => offset,
            Scroll::MaybeBottom(offset) => {
                if offset < self.lines.saturating_sub(self.window_lines) {
                    self.scroll = Scroll::Offset(offset);
                    offset
                } else {
                    self.scroll = Scroll::Bottom;
                    self.lines.saturating_sub(self.window_lines)
                }
            }
            Scroll::Bottom => self.lines.saturating_sub(self.window_lines),
        };

        let start_line = scroll_offset;
        let end_line = scroll_offset
            .saturating_add(self.window_lines)
            .min(self.lines);

        let items: Vec<Line> = all_lines
            .iter()
            .skip(start_line)
            .take(end_line - start_line)
            .map(|line| Line::from(line.clone()))
            .collect();

        let mut scrollbar_area = area;

        let mut paragraph = Paragraph::new(items)
            .alignment(Alignment::Left)
            .style(self.style);

        if let Some(block) = &self.block {
            paragraph = paragraph.block(block.clone());
            scrollbar_area.y = block.inner(area).y;
            scrollbar_area.height = block.inner(area).height;
        }

        // render the paragraph
        Widget::render(paragraph, area, buf);

        // render the scrollbar if needed
        if self.lines > self.window_lines {
            let mut scrollbar_state =
                ScrollbarState::new(self.lines.saturating_sub(self.window_lines))
                    .position(scroll_offset)
                    .viewport_content_length(self.window_lines);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .track_symbol(Some("│"))
                .thumb_symbol("█")
                .end_symbol(Some("↓"));
            StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut scrollbar_state);
        }
    }
}
