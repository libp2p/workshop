use crate::{
    languages::spoken,
    ui::tui::{
        self,
        screens::{self, Screens},
        widgets::ScrollText,
        Screen,
    },
    Error, Status,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    symbols::border::Set,
    text::{Line, Span},
    widgets::{block::Position, Block, Borders, Clear, Padding, StatefulWidget, Widget},
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use tracing::info;

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
pub struct License<'a> {
    /// license text
    text: String,
    /// the cached rect from last render
    area: Rect,
    /// the cached calculated rect
    centered: Rect,
    /// scroll text widget
    st: ScrollText<'a>,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
}

impl License<'_> {
    /// Create a new log Screen
    pub fn new() -> Self {
        let mut st = ScrollText::default();
        st.scroll_top();
        Self {
            text: String::new(),
            area: Rect::default(),
            centered: Rect::default(),
            st,
            spoken_language: None,
        }
    }

    /// set the license text
    pub async fn set_license(
        &mut self,
        text: String,
        spoken_language: Option<spoken::Code>,
    ) -> Result<(), Error> {
        self.text = text;
        self.spoken_language = spoken_language;
        Ok(())
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
            self.area = area;
        }
    }

    // render the log messages
    fn render_license(&mut self, area: Rect, buf: &mut Buffer) {
        Widget::render(Clear, area, buf);

        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled("/ License /", Style::default().fg(Color::White)),
        ]);

        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(Color::White))
            .padding(Padding::horizontal(1))
            .style(Style::default().fg(Color::DarkGray))
            .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
            .border_set(TOP_DIALOG_BORDER);

        self.st.block(block);
        self.st.style(Style::default().fg(Color::White));

        // render the scroll text
        StatefulWidget::render(&mut self.st, area, buf, &mut self.text);
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        let line = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "/ j,k scroll / ⤒ top / ⤓ bottom / b back / q quit /",
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
            tui::Event::ShowLicense(text) => {
                info!("Setting license text");
                let spoken = {
                    let status = status.lock().unwrap();
                    status.spoken_language()
                };
                self.set_license(text, spoken).await?;
                to_ui
                    .send((None, tui::Event::Show(Screens::License)).into())
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
        if let event::Event::Key(key) = event {
            match key.code {
                KeyCode::PageUp => self.st.scroll_top(),
                KeyCode::PageDown => self.st.scroll_bottom(),
                KeyCode::Char('b') | KeyCode::Esc => {
                    to_ui
                        .send((Some(Screens::Workshops), tui::Event::LoadWorkshops).into())
                        .await?;
                }
                KeyCode::Char('j') | KeyCode::Down => self.st.scroll_down(),
                KeyCode::Char('k') | KeyCode::Up => self.st.scroll_up(),
                _ => {}
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Screen for License<'_> {
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

        let [license_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(self.centered);

        self.render_license(license_area, buf);
        self.render_status(status_area, buf);
        Ok(())
    }
}
