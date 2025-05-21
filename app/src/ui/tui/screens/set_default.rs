use crate::{
    ui::tui::{Event as UiEvent, Screen},
    Error,
};
use crossterm::event::{Event, KeyCode};
use engine::Message;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Offset, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Clear, List, ListState, Padding, Paragraph, StatefulWidget, Widget, Wrap,
    },
};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tracing::info;

#[derive(Clone, Debug)]
pub struct SetDefault<'a> {
    /// the value to set as default
    event: UiEvent,
    /// the cached rect from last render
    area: Rect,
    /// the cached calculated rect
    centered: Rect,
    /// the cached list
    list: List<'a>,
    /// programming language list state
    list_state: ListState,
}

impl SetDefault<'_> {
    /// create a new yes/no dialog
    pub fn new(question: &str) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let list = List::new(vec!["Yes", "No"])
            .block(
                Block::default()
                    .title(format!(" {} ", question))
                    .padding(Padding::horizontal(1))
                    .style(Style::default().fg(Color::White))
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::White))
            .highlight_symbol("> ");

        Self {
            event: UiEvent::Back,
            area: Rect::default(),
            centered: Rect::default(),
            list,
            list_state,
        }
    }

    // set the value to return if they select yet
    fn set_value(&mut self, event: UiEvent) {
        self.event = event;
    }

    fn recalculate_rect(&mut self, area: Rect) {
        if self.area != area {
            let [_, hc, _] = Layout::horizontal([
                Constraint::Percentage(10),
                Constraint::Min(1),
                Constraint::Percentage(10),
            ])
            .areas(area);
            [_, self.centered, _] = Layout::vertical([
                Constraint::Percentage(10),
                Constraint::Min(1),
                Constraint::Percentage(10),
            ])
            .areas(hc);
        }
    }

    // render the list
    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        // clear popup area
        Widget::render(Clear, area, buf);

        // render the list of programming language names
        StatefulWidget::render(&self.list, area, buf, &mut self.list_state);
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(1));

        let keys = Paragraph::new(" ↓/↑ or j/k: scroll  |  enter: select ")
            .block(block)
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);

        Widget::render(keys, area, buf);
    }
}

#[async_trait::async_trait]
impl Screen for SetDefault<'_> {
    /// handle an input event
    async fn handle_event(
        &mut self,
        evt: Event,
        _to_engine: Sender<Message>,
    ) -> Result<Option<UiEvent>, Error> {
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => self.list_state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Enter => {
                    match self.list_state.selected() {
                        Some(0) => return Ok(Some(self.event.clone())),
                        Some(_) | None => return Ok(Some(UiEvent::Back)),
                    };
                }
                _ => {}
            }
        }
        Ok(None)
    }

    async fn handle_message(
        &mut self,
        msg: Message,
        _to_engine: Sender<Message>,
    ) -> Result<Option<UiEvent>, Error> {
        if let Message::SetSpokenLanguageDefault { spoken_language } = msg {
            info!("Set default spoken language?: {:?}", spoken_language);
            self.set_value(UiEvent::SetSpokenLanguageDefault { spoken_language });
        }
        Ok(None)
    }

    fn render_screen(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        _last_frame_duration: Duration,
    ) -> Result<(), Error> {
        self.recalculate_rect(area);

        // clear area around the popup
        Widget::render(Clear, self.centered, buf);

        let centered_block = Block::default()
            .padding(Padding::uniform(2))
            .borders(Borders::NONE);
        let working_area = centered_block.inner(self.centered);

        // draw drop shadow
        let mut shadow_area = working_area;
        shadow_area = shadow_area.offset(Offset { x: 1, y: 1 });
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray).bg(Color::DarkGray));
        Widget::render(block, shadow_area, buf);

        let [question_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(working_area);

        self.render_list(question_area, buf);
        self.render_status(status_area, buf);
        Ok(())
    }
}
