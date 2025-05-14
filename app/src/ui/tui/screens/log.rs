use crate::{
    ui::tui::{Event as UiEvent, EventHandler},
    Error,
};
use crossterm::event::{Event, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};
use std::collections::VecDeque;
use textwrap::{self, Options};

#[derive(Clone, Debug, Default)]
pub struct Log {
    /// log lines
    log: VecDeque<String>,
    /// selected
    selected: Option<usize>,
    /// item count
    items: usize,
}

impl Log {
    /// set the log
    pub fn add_message(&mut self, message: String) {
        self.log.push_back(message);
        while self.log.len() > 1000 {
            self.log.pop_front();
        }
    }

    // render the log messages
    fn render_log(&mut self, area: Rect, buf: &mut Buffer) {
        // render the list of log lines
        let block = Block::new()
            .title(Line::from(" Log "))
            .padding(Padding::horizontal(1))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        // wrap the log messages
        let wrap_options = Options::new(block.inner(area).width.into()).subsequent_indent("  ");
        let mut total_lines = 0;
        let wrapped_items = self
            .log
            .iter()
            .map(|line| {
                let lines = textwrap::wrap(line, &wrap_options)
                    .into_iter()
                    .map(|cow| Line::from(cow.into_owned()))
                    .collect::<Vec<_>>();
                total_lines += lines.len();
                ListItem::new(lines)
            })
            .collect::<Vec<_>>();
        self.items = total_lines;

        // build the list
        let list = List::new(wrapped_items)
            .block(block.clone())
            .scroll_padding(2);

        let mut list_state = ListState::default();
        let selected = if let Some(selected) = self.selected {
            list_state.select(Some(selected));
            selected
        } else {
            list_state.select(Some(self.items.saturating_sub(1)));
            self.items.saturating_sub(1)
        };

        // clear
        Widget::render(Clear, area, buf);

        StatefulWidget::render(list, area, buf, &mut list_state);

        let window_lines = block.inner(area).height as usize;

        // only render the scrollbar when needed
        if total_lines > window_lines {
            // set up the scrolbar state
            let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(window_lines))
                .position(selected.saturating_sub(window_lines))
                .viewport_content_length(window_lines);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            StatefulWidget::render(scrollbar, area, buf, &mut scrollbar_state);
        }
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        let keys = Paragraph::new(
            " ↓/↑ or j/k: scroll  |  PgUp: start  | PgDwn: end  |  b: back  |  q: quit",
        )
        .style(Style::default().fg(Color::Black).bg(Color::White))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

        Widget::render(keys, area, buf);
    }
}

#[async_trait::async_trait]
impl EventHandler for &mut Log {
    /// handle an input event
    async fn handle_event(&mut self, evt: Event) -> Result<UiEvent, Error> {
        match evt {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => Ok(UiEvent::Quit),
                KeyCode::Char('b') | KeyCode::Esc => Ok(UiEvent::ClosePopup),
                KeyCode::PageUp => {
                    self.selected = Some(0);
                    Ok(UiEvent::Noop)
                }
                KeyCode::PageDown => {
                    self.selected = None;
                    Ok(UiEvent::Noop)
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if let Some(selected) = self.selected {
                        if selected + 1 >= self.items {
                            self.selected = None;
                        } else {
                            self.selected = Some(selected + 1);
                        }
                    } else {
                        self.selected = Some(self.items);
                    }
                    Ok(UiEvent::Noop)
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if let Some(selected) = self.selected {
                        self.selected = Some(selected.saturating_sub(1));
                    } else {
                        self.selected = Some(self.items - 1);
                    }
                    Ok(UiEvent::Noop)
                }
                _ => Ok(UiEvent::Noop),
            },
            _ => Ok(UiEvent::Noop),
        }
    }
}

impl Widget for &mut Log {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [_, hc, _] = Layout::horizontal([
            Constraint::Percentage(10),
            Constraint::Min(1),
            Constraint::Percentage(10),
        ])
        .areas(area);
        let [_, centered, _] = Layout::vertical([
            Constraint::Percentage(10),
            Constraint::Min(1),
            Constraint::Percentage(10),
        ])
        .areas(hc);

        let [log_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(centered);

        self.render_log(log_area, buf);
        self.render_status(status_area, buf);
    }
}
