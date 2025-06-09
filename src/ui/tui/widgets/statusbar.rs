use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Gauge, Paragraph, Widget},
};
use std::time::Instant;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum StatusMode {
    #[default]
    Blank,
    Messages,
    Progress,
}

/// A status bar widget that can display messages with a throbber or progress bars
#[derive(Clone, Debug, Default)]
pub struct StatusBar<'a> {
    /// Current mode of the status bar
    mode: StatusMode,
    /// Current message to display
    message: String,
    /// Progress percentage (0-100)
    progress: u8,
    /// Start time for throbber animation
    start_time: Option<Instant>,
    /// the block to render with
    block: Block<'a>,
}

impl<'a> StatusBar<'a> {
    /// Create a new StatusBar
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the mode to Blank (blank)
    pub fn set_blank(&mut self) {
        self.mode = StatusMode::Blank;
        self.message.clear();
        self.progress = 0;
        self.start_time = None;
    }

    /// Set the mode to Messages with a command
    pub fn set_messages(&mut self, message: String) {
        self.mode = StatusMode::Messages;
        self.message = message;
        self.progress = 0;
        self.start_time = Some(Instant::now());
    }

    /// Set the mode to Progress with initial message
    pub fn set_progress(&mut self, message: String) {
        self.mode = StatusMode::Progress;
        self.message = message;
        self.progress = 0;
        self.start_time = Some(Instant::now());
    }

    /// Set the block to render with
    pub fn set_block(&mut self, block: Block<'a>) {
        self.block = block;
    }

    /// Update message (for Messages mode)
    pub fn update_message(&mut self, message: String) {
        if self.mode == StatusMode::Messages {
            self.message = message;
        }
    }

    /// Update progress and message (for Progress mode)
    pub fn update_progress(&mut self, message: Option<String>, progress: u8) {
        if self.mode == StatusMode::Progress {
            if let Some(msg) = message {
                self.message = msg;
            }
            self.progress = progress.min(100);
        }
    }

    /// Get current throbber character based on elapsed time
    fn get_throbber_char(&self) -> char {
        if let Some(start_time) = self.start_time {
            let elapsed = start_time.elapsed();
            let frame = (elapsed.as_millis() / 100) % 10; // 100ms per frame, 10 frames
            match frame {
                0 => '⠋',
                1 => '⠙',
                2 => '⠹',
                3 => '⠸',
                4 => '⠼',
                5 => '⠴',
                6 => '⠦',
                7 => '⠧',
                8 => '⠇',
                9 => '⠏',
                _ => '⠋',
            }
        } else {
            '⠋'
        }
    }
}

impl Widget for &mut StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.mode {
            StatusMode::Blank => {
                // Render blank line
                let paragraph = Paragraph::new(Line::from("")).block(self.block.clone());
                Widget::render(paragraph, area, buf);
            }
            StatusMode::Messages => {
                // Render throbber + message
                let throbber = self.get_throbber_char();
                let content = format!("{} {}", throbber, self.message);
                let paragraph = Paragraph::new(Line::from(content))
                    .block(self.block.clone())
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(Alignment::Left);
                Widget::render(paragraph, area, buf);
            }
            StatusMode::Progress => {
                // Render progress bar with throbber and message
                let throbber = self.get_throbber_char();
                let label = format!("{} {}", throbber, self.message);

                let gauge = Gauge::default()
                    .block(self.block.clone())
                    .gauge_style(Style::default().fg(Color::Green))
                    .percent(self.progress as u16)
                    .label(label);

                Widget::render(gauge, area, buf);
            }
        }
    }
}
