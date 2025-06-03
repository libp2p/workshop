use crate::{
    languages::spoken,
    ui::tui::{self, screens, Screen},
    Error,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Offset, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Clear, List, ListState, Padding, Paragraph, StatefulWidget, Widget, Wrap,
    },
};
use tokio::sync::mpsc::Sender;
use tracing::info;

#[derive(Clone, Debug)]
pub struct SetDefault<'a> {
    /// the cached rect from last render
    area: Rect,
    /// the cached calculated rect
    centered: Rect,
    /// the cached list
    list: List<'a>,
    /// programming language list state
    list_state: ListState,
    /// event to send if they select "yes"
    event: tui::Event,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
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
            area: Rect::default(),
            centered: Rect::default(),
            list,
            list_state,
            event: tui::Event::Show(screens::Screens::Workshops),
            spoken_language: None,
        }
    }

    async fn set_spoken_language(
        &mut self,
        spoken_language: Option<spoken::Code>,
    ) -> Result<(), Error> {
        self.spoken_language = spoken_language;
        Ok(())
    }

    async fn set_event(&mut self, event: tui::Event) -> Result<(), Error> {
        self.event = event;
        Ok(())
    }

    fn recalculate_rect(&mut self, area: Rect) {
        if self.area != area {
            let [_, hc, _] = Layout::horizontal([
                Constraint::Fill(1),
                Constraint::Max(44),
                Constraint::Fill(1),
            ])
            .areas(area);
            [_, self.centered, _] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Max(10),
                Constraint::Fill(1),
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

    /// handle UI events
    pub async fn handle_ui_event(
        &mut self,
        event: tui::Event,
        _to_ui: Sender<screens::Event>,
    ) -> Result<(), Error> {
        match event {
            tui::Event::SpokenLanguage(spoken_language) => {
                info!("Spoken language set: {:?}", spoken_language);
                self.set_spoken_language(spoken_language).await?;
            }
            tui::Event::SetEvent(event) => {
                info!("Setting event: {:?}", event);
                self.set_event(*event).await?;
            }
            _ => {
                info!("Ignoring UI event: {:?}", event);
            }
        }
        Ok(())
    }

    /// handle input events
    pub async fn handle_input_event(
        &mut self,
        event: event::Event,
        to_ui: Sender<screens::Event>,
    ) -> Result<(), Error> {
        if let event::Event::Key(key) = event {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => self.list_state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Enter => {
                    match self.list_state.selected() {
                        Some(0) => {
                            info!("Setting default");
                            to_ui.send(self.event.clone().into()).await?;
                        }
                        Some(_) | None => {
                            info!("Back to previous screen");
                            to_ui.send(tui::Event::LoadWorkshops.into()).await?;
                        }
                    };
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Screen for SetDefault<'_> {
    async fn handle_event(
        &mut self,
        event: screens::Event,
        to_ui: Sender<screens::Event>,
    ) -> Result<(), Error> {
        match event {
            screens::Event::Input(input_event) => self.handle_input_event(input_event, to_ui).await,
            screens::Event::Ui(ui_event) => self.handle_ui_event(ui_event, to_ui).await,
        }
    }

    fn render_screen(&mut self, area: Rect, buf: &mut Buffer) -> Result<(), Error> {
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
