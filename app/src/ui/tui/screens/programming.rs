use crate::{
    ui::tui::{Event as UiEvent, Screen},
    Error,
};
use crossterm::event::{Event, KeyCode};
use engine::Message;
use languages::programming;
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

#[derive(Clone, Debug, Default)]
pub struct Programming<'a> {
    /// the programming language list
    programming_languages: Vec<programming::Code>,
    /// whether their selection sets the default value
    set_default: bool,
    /// the cached rect from last render
    area: Rect,
    /// the cached calculated rect
    centered: Rect,
    /// the cached list
    list: List<'a>,
    /// programming language list state
    list_state: ListState,
}

impl Programming<'_> {
    /// set the programming language list
    pub fn set_programming_languages(
        &mut self,
        programming_languages: &[programming::Code],
        set_default: bool,
    ) {
        self.programming_languages = programming_languages.to_vec();
        self.set_default = set_default;
        let programming_language_names = programming_languages
            .iter()
            .map(|code| code.get_name().to_string())
            .collect::<Vec<_>>();
        self.list_state.select(Some(0));
        self.list = List::new(programming_language_names)
            .block(
                Block::default()
                    .title(" Programming Languages ")
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

        let keys = Paragraph::new(
            " ↓/↑ or j/k: scroll  |  enter: select  |  PgUp: start  | PgDwn: end  |  b: back  |  q: quit",
        )
        .block(block)
        .style(Style::default().fg(Color::Black).bg(Color::White))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

        Widget::render(keys, area, buf);
    }
}

#[async_trait::async_trait]
impl Screen for Programming<'_> {
    /// handle an input event
    async fn handle_event(
        &mut self,
        evt: Event,
        to_engine: Sender<Message>,
    ) -> Result<Option<UiEvent>, Error> {
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::PageUp => self.list_state.select_first(),
                KeyCode::PageDown => self.list_state.select_last(),
                KeyCode::Char('b') | KeyCode::Esc => return Ok(Some(UiEvent::Back)),
                KeyCode::Char('j') | KeyCode::Down => self.list_state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Enter => {
                    if let Some(selected) = self.list_state.selected() {
                        if let Some(code) = self.programming_languages.get(selected) {
                            info!("Programming language selected: {:?}", code);
                            to_engine
                                .send(Message::SetProgrammingLanguage { code: *code })
                                .await?;
                            return Ok(Some(UiEvent::SetProgrammingLanguage {
                                code: *code,
                                set_default: self.set_default,
                            }));
                        }
                    }
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
        if let Message::SelectProgrammingLanguage {
            programming_languages,
            set_default,
        } = msg
        {
            info!("Select programming language: {:?}", programming_languages);
            self.set_programming_languages(&programming_languages, set_default);
            return Ok(Some(UiEvent::SelectProgrammingLanguage));
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

        let [log_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(working_area);

        self.render_list(log_area, buf);
        self.render_status(status_area, buf);
        Ok(())
    }
}
