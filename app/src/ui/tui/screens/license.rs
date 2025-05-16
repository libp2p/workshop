use crate::{
    ui::tui::{widgets::ScrollText, Event as UiEvent, EventHandler},
    Error,
};
use crossterm::event::{Event, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Offset, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Padding, Paragraph, StatefulWidget, Widget, Wrap},
};

#[derive(Clone, Debug, Default)]
pub struct License<'a> {
    /// scroll text widget
    st: ScrollText<'a>,
}

impl License<'_> {
    // render the log messages
    fn render_license(&mut self, area: Rect, buf: &mut Buffer, text: &str) {
        // clear popup area
        Widget::render(Clear, area, buf);

        // render the list of license lines
        let block = Block::new()
            .title(Line::from(" License "))
            .padding(Padding::horizontal(1))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        self.st.block(block);
        self.st
            .style(Style::default().fg(Color::White).bg(Color::Black));

        // render the scroll text
        StatefulWidget::render(&mut self.st, area, buf, &mut text.to_string());
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(1));

        let keys = Paragraph::new(
            " ↓/↑ or j/k: scroll  |  PgUp: start  | PgDwn: end  |  b: back  |  q: quit",
        )
        .block(block)
        .style(Style::default().fg(Color::Black).bg(Color::White))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

        Widget::render(keys, area, buf);
    }
}

#[async_trait::async_trait]
impl EventHandler for &mut License<'_> {
    /// handle an input event
    async fn handle_event(&mut self, evt: &Event) -> Result<Option<UiEvent>, Error> {
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::PageUp => self.st.scroll_top(),
                KeyCode::PageDown => self.st.scroll_bottom(),
                KeyCode::Char('j') | KeyCode::Down => self.st.scroll_down(),
                KeyCode::Char('k') | KeyCode::Up => self.st.scroll_up(),
                _ => {}
            }
        }
        Ok(None)
    }
}

impl StatefulWidget for &mut License<'_> {
    type State = String;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
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

        // clear area around the popup
        Widget::render(Clear, centered, buf);

        let centered_block = Block::default()
            .padding(Padding::uniform(2))
            .borders(Borders::NONE);
        let working_area = centered_block.inner(centered);

        // draw drop shadow
        let mut shadow_area = working_area;
        shadow_area = shadow_area.offset(Offset { x: 1, y: 1 });
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray).bg(Color::DarkGray));
        Widget::render(block, shadow_area, buf);

        let [log_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(working_area);

        self.render_license(log_area, buf, state);
        self.render_status(status_area, buf);
    }
}
