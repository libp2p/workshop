use crate::{
    languages::spoken,
    ui::tui::{self, screens, Screen},
    Error, Status,
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
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use tracing::info;

#[derive(Clone, Debug, Default)]
pub struct SetDefault<'a> {
    /// the title
    title: String,
    /// the cached rect from last render
    area: Rect,
    /// the cached calculated rect
    centered: Rect,
    /// the cached list
    list: List<'a>,
    /// programming language list state
    list_state: ListState,
    /// event to send if they select "yes"
    event: Option<Box<tui::Event>>,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
}

impl SetDefault<'_> {
    async fn init(
        &mut self,
        title: &str,
        spoken_language: Option<spoken::Code>,
        event: Option<Box<tui::Event>>,
    ) -> Result<(), Error> {
        self.title = title.to_string();
        self.spoken_language = spoken_language;
        self.event = event;

        self.list_state.select(Some(0));
        self.list = List::new(vec!["Yes", "No"])
            .block(
                Block::default()
                    .title(format!(" {} ", self.title))
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
        to_ui: Sender<screens::Event>,
        status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        match event {
            tui::Event::SetDefault(title, event) => {
                info!("Set as default?");
                let spoken = {
                    let status = status.lock().unwrap();
                    status.spoken_language()
                };
                self.init(&title, spoken, event).await?;
                info!("Showing SetDefault screen");
                to_ui
                    .send((None, tui::Event::Show(screens::Screens::SetDefault)).into())
                    .await?;
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
        _status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        info!("SetDefault input event: {:?}", event);
        if let event::Event::Key(key) = event {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => self.list_state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Enter => {
                    match self.list_state.selected() {
                        Some(0) => {
                            if let Some(event) = self.event.take() {
                                info!("Setting default: {:?}", event);
                                to_ui.send((None, *event).into()).await?;
                            } else {
                                info!("No event to send");
                            }
                        }
                        Some(_) | None => {
                            info!("Back to previous screen");
                            to_ui
                                .send(
                                    (Some(screens::Screens::Workshops), tui::Event::LoadWorkshops)
                                        .into(),
                                )
                                .await?;
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
        status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        match event {
            screens::Event::Input(input_event) => {
                self.handle_input_event(input_event, to_ui, status).await
            }
            screens::Event::Ui(_, ui_event) => self.handle_ui_event(ui_event, to_ui, status).await,
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
