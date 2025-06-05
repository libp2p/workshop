use crate::{
    languages::spoken,
    ui::tui::{
        self,
        screens::{self, Screens},
        Evt, Screen,
    },
    Error, Status,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::border::Set,
    text::{Line, Span},
    widgets::{
        block::Position, Block, Borders, Clear, List, ListState, Padding, StatefulWidget, Widget,
    },
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use tracing::{debug, info};

const TOP_DIALOG_BORDER: Set = Set {
    top_left: "┌",
    top_right: "┐",
    bottom_left: "│",
    bottom_right: "│",
    vertical_left: "│",
    vertical_right: "│",
    horizontal_top: "─",
    horizontal_bottom: " ",
};

const STATUS_BORDER: Set = Set {
    top_left: " ",
    top_right: " ",
    bottom_left: "└",
    bottom_right: "┘",
    vertical_left: " ",
    vertical_right: " ",
    horizontal_top: " ",
    horizontal_bottom: "─",
};

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
    yes: Option<Evt>,
    /// event to send if they select "no"
    no: Option<Evt>,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
}

impl SetDefault<'_> {
    async fn init(
        &mut self,
        title: &str,
        spoken_language: Option<spoken::Code>,
        yes: Option<Evt>,
        no: Option<Evt>,
    ) -> Result<(), Error> {
        self.title = title.to_string();
        self.spoken_language = spoken_language;
        self.yes = yes;
        self.no = no;

        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("/ {} /", self.title),
                Style::default().fg(Color::White),
            ),
        ]);
        self.list_state.select(Some(0));
        self.list = List::new(vec!["Yes", "No"])
            .block(
                Block::default()
                    .title(title)
                    .title_style(Style::default().fg(Color::White))
                    .padding(Padding::uniform(1))
                    .style(Style::default().fg(Color::DarkGray))
                    .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
                    .border_set(TOP_DIALOG_BORDER),
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
                Constraint::Length(6),
                Constraint::Fill(1),
            ])
            .areas(hc);
            self.area = area;
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
        let line = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "/ j,k scroll / ↵ select /",
                Style::default().fg(Color::White),
            ),
        ]);
        let block = Block::default()
            .title(line)
            .title_style(Style::default().fg(Color::White))
            .title_position(Position::Bottom)
            .title_alignment(Alignment::Left)
            .style(Style::default().fg(Color::DarkGray))
            .borders(Borders::LEFT | Borders::BOTTOM | Borders::RIGHT)
            .border_set(STATUS_BORDER)
            .padding(Padding::horizontal(1));

        Widget::render(block, area, buf);
    }

    /// handle UI events
    pub async fn handle_ui_event(
        &mut self,
        event: tui::Event,
        to_ui: Sender<screens::Event>,
        status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        match event {
            tui::Event::SetDefault(title, yes, no) => {
                info!("Set as default?");
                let spoken = {
                    let status = status.lock().unwrap();
                    status.spoken_language()
                };
                self.init(&title, spoken, yes, no).await?;
                to_ui
                    .send((None, tui::Event::Show(Screens::SetDefault)).into())
                    .await?;
            }
            _ => {
                debug!("Ignoring UI event: {:?}", event);
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
        if let event::Event::Key(key) = event {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => self.list_state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Enter => {
                    // take the events leaving None in their place
                    let yes = self.yes.take();
                    let no = self.no.take();
                    match self.list_state.selected() {
                        Some(0) => {
                            if let Some(yes) = yes {
                                debug!("Setting default: {:?}", yes);
                                to_ui.send(yes.into()).await?;
                            }
                        }
                        Some(_) | None => {
                            if let Some(no) = no {
                                debug!("Clearing default: {:?}", no);
                                to_ui.send(no.into()).await?;
                            }
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

        let [list_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(self.centered);

        self.render_list(list_area, buf);
        self.render_status(status_area, buf);
        Ok(())
    }
}
