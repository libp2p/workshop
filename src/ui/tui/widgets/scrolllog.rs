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
    /// Show oldest messages (at top of screen)
    Oldest,
    /// Maybe scroll to oldest messages
    MaybeOldest(usize),
    /// Fixed scroll offset from newest
    Offset(usize),
    /// Maybe scroll to newest messages
    MaybeNewest(usize),
    /// Show newest messages (at bottom of screen) - default for logs
    #[default]
    Newest,
}

impl fmt::Display for Scroll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scroll::Oldest => write!(f, "Oldest"),
            Scroll::MaybeOldest(offset) => write!(f, "MaybeOldest({offset})"),
            Scroll::Offset(offset) => write!(f, "Offset({offset})"),
            Scroll::MaybeNewest(offset) => write!(f, "MaybeNewest({offset})"),
            Scroll::Newest => write!(f, "Newest"),
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

    /// Scroll to the oldest messages
    pub fn scroll_oldest(&mut self) {
        self.scroll = Scroll::Oldest;
    }

    /// Scroll to the newest messages
    pub fn scroll_newest(&mut self) {
        self.scroll = Scroll::Newest;
    }

    /// Scroll toward older messages
    pub fn scroll_older(&mut self) {
        match self.scroll {
            Scroll::Newest => {
                // Start scrolling toward older messages from newest (if there's room)
                self.scroll = Scroll::MaybeOldest(1);
            }
            Scroll::Offset(offset) => {
                // Continue scrolling toward older messages (increasing offset from newest)
                self.scroll = Scroll::MaybeOldest(offset.saturating_add(1));
            }
            _ => {}
        }
    }

    /// Scroll toward newer messages
    pub fn scroll_newer(&mut self) {
        match self.scroll {
            Scroll::Oldest => {
                // Start scrolling toward newer messages from oldest
                self.scroll = Scroll::MaybeNewest(1);
            }
            Scroll::Offset(offset) => {
                // Continue scrolling toward newer messages (decreasing offset from newest)
                self.scroll = Scroll::MaybeNewest(offset.saturating_sub(1));
            }
            _ => {}
        }
    }
}

impl StatefulWidget for &mut ScrollLog<'_> {
    type State = VecDeque<(Option<String>, String)>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // ScrollLog now renders in the full area - StatusBar will be rendered separately
        let log_messages = state;
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

        for (emoji, message) in log_messages.iter() {
            let wrap_options = textwrap::Options::new(right_column_width).break_words(true);
            let wrapped_lines = textwrap::wrap(message, &wrap_options);

            // first line includes the emoji
            if let Some(first_line) = wrapped_lines.first() {
                if let Some(emoji_str) = emoji {
                    all_lines.push(format!("{emoji_str:<2}{first_line}"));
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

        // figure out the scroll offset (from the end for bottom-up display)
        let scroll_offset_from_end = match self.scroll {
            Scroll::Oldest => {
                // Show oldest messages (from beginning of all_lines)
                self.lines.saturating_sub(self.window_lines)
            }
            Scroll::MaybeOldest(offset) => {
                let max_offset = self.lines.saturating_sub(self.window_lines);
                if offset <= max_offset {
                    self.scroll = Scroll::Offset(offset);
                    offset
                } else {
                    self.scroll = Scroll::Oldest;
                    max_offset
                }
            }
            Scroll::Offset(offset) => {
                let max_offset = self.lines.saturating_sub(self.window_lines);
                offset.min(max_offset)
            }
            Scroll::MaybeNewest(offset) => {
                if offset > 0 {
                    self.scroll = Scroll::Offset(offset);
                    offset
                } else {
                    self.scroll = Scroll::Newest;
                    0
                }
            }
            Scroll::Newest => 0, // Show newest messages (no offset from end)
        };

        // Calculate which lines to show (from the end for bottom-up)
        let start_line = if self.lines > self.window_lines {
            self.lines - self.window_lines - scroll_offset_from_end
        } else {
            0
        };
        let end_line = start_line.saturating_add(self.window_lines).min(self.lines);

        // Get the selected lines
        let selected_lines: Vec<String> = all_lines
            .iter()
            .skip(start_line)
            .take(end_line - start_line)
            .cloned()
            .collect();

        // For bottom-up rendering, we need to pad with empty lines at the top
        // if we have fewer lines than the window height
        let mut items: Vec<Line> = Vec::new();

        // Add empty lines at the top to push content to bottom
        let lines_to_render = selected_lines.len();
        if lines_to_render < self.window_lines {
            let empty_lines_needed = self.window_lines - lines_to_render;
            for _ in 0..empty_lines_needed {
                items.push(Line::from(""));
            }
        }

        // Add the actual log lines
        for line in selected_lines {
            items.push(Line::from(line));
        }

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
                    .position(scroll_offset_from_end)
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
