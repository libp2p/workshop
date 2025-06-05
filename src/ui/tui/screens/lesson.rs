use crate::{
    fs,
    languages::{programming, spoken},
    ui::tui::{self, screens, widgets::ScrollText, Screen},
    Error, Status,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    symbols::border::Set,
    text::{Line, Span},
    widgets::{block::Position, Block, Borders, Padding, StatefulWidget, Widget},
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use tracing::info;

const TOP_BORDER: Set = Set {
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
    vertical_left: "│",
    vertical_right: "│",
    horizontal_top: " ",
    horizontal_bottom: "─",
};

#[derive(Clone, Debug, Default)]
pub struct Lesson<'a> {
    /// the title of the workshop
    workshop_title: String,
    /// the title of the lesson
    lesson_title: String,
    /// the lesson text
    text: String,
    /// the scrollable info window - requires lifetime for Block
    st: ScrollText<'a>,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
    /// the currently selected programming language
    programming_language: Option<programming::Code>,
}

impl Lesson<'_> {
    /// set the lessons
    async fn init<S: AsRef<str>>(
        &mut self,
        workshop_title: S,
        lesson_title: S,
        text: S,
        spoken_language: Option<spoken::Code>,
        programming_language: Option<programming::Code>,
    ) -> Result<(), Error> {
        self.workshop_title = workshop_title.as_ref().to_string();
        self.lesson_title = lesson_title.as_ref().to_string();
        self.text = text.as_ref().to_string();
        self.spoken_language = spoken_language;
        self.programming_language = programming_language;
        Ok(())
    }

    /// render the lesson
    fn render_lesson(&mut self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray).bg(Color::Black)),
            Span::styled(
                format!("/ {} /", self.lesson_title),
                Style::default().fg(Color::White).bg(Color::Black),
            ),
        ]);
        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(Color::White).bg(Color::Black))
            .padding(Padding::uniform(1))
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black))
            .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
            .border_set(TOP_BORDER);

        self.st.block(block);
        self.st
            .style(Style::default().fg(Color::White).bg(Color::Black));

        // render the scroll text
        StatefulWidget::render(&mut self.st, area, buf, &mut self.text);
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        // render the status bar at the bottom
        let [keys_area, langs_area] =
            Layout::horizontal([Constraint::Min(1), Constraint::Length(40)]).areas(area);

        self.render_keys(keys_area, buf);
        self.render_langs(langs_area, buf);
    }

    // render the keyboard shortcuts
    fn render_keys(&mut self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray).bg(Color::Black)),
            Span::styled(
                "/ j,k scroll / ⇥ next hint / ↵ expand hint / c check / b back / q quit /",
                Style::default().fg(Color::White).bg(Color::Black),
            ),
        ]);
        let block = Block::default()
            .title(title)
            .title_style(Style::default().bg(Color::Black).fg(Color::White))
            .title_position(Position::Bottom)
            .title_alignment(Alignment::Left)
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black))
            .borders(Borders::LEFT | Borders::BOTTOM)
            .border_set(STATUS_BORDER)
            .padding(Padding::horizontal(1));

        Widget::render(block, area, buf);
    }

    // render the selected languages
    fn render_langs(&mut self, area: Rect, buf: &mut Buffer) {
        let spoken = match self.spoken_language {
            Some(code) => code.get_name_in_english().to_string(),
            None => "All".to_string(),
        };

        let programming = match self.programming_language {
            Some(code) => code.get_name().to_string(),
            None => "All".to_string(),
        };

        let title = Line::from(vec![
            Span::styled(
                format!("/ {} / {spoken} / {programming} /", self.workshop_title),
                Style::default().fg(Color::White).bg(Color::Black),
            ),
            Span::styled("─", Style::default().fg(Color::DarkGray).bg(Color::Black)),
        ]);

        let block = Block::default()
            .title(title)
            .title_style(Style::default().bg(Color::Black).fg(Color::White))
            .title_position(Position::Bottom)
            .title_alignment(Alignment::Right)
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black))
            .borders(Borders::RIGHT | Borders::BOTTOM)
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
            tui::Event::LoadLesson => {
                info!("Loading lessons");
                let (spoken, programming, workshop, lesson) = {
                    let status = status
                        .lock()
                        .map_err(|e| Error::StatusLock(e.to_string()))?;
                    (
                        status.spoken_language(),
                        status.programming_language(),
                        status.workshop().unwrap(),
                        status.lesson().unwrap(),
                    )
                };
                if let Some(workshop_data) = fs::workshops::load(&workshop) {
                    info!("Loading lessons for workshop: {}", &workshop);
                    let lessons = workshop_data.get_lessons_data(spoken, programming).await?;
                    let workshop_title = workshop_data.get_metadata(spoken).await?.title;
                    let lesson_data = lessons.get(&lesson).unwrap();
                    let lesson_text = lesson_data.get_text().await?;
                    let lesson_title = lesson_data.get_metadata().await?.title;
                    self.init(
                        &workshop_title,
                        &lesson_title,
                        &lesson_text,
                        spoken,
                        programming,
                    )
                    .await?;
                    to_ui
                        .send((None, tui::Event::Show(screens::Screens::Lesson)).into())
                        .await?;
                } else {
                    info!("Failed to load workshop data for: {}", &workshop);
                }
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
                KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => self.st.scroll_down(),
                KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => self.st.scroll_up(),
                KeyCode::Char('b') | KeyCode::Esc => {
                    to_ui
                        .send((None, tui::Event::SetLesson(None)).into())
                        .await?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Screen for Lesson<'_> {
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
        // this splits the screen into a top area and a one-line bottom area
        let [lesson_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(area);

        self.render_lesson(lesson_area, buf);
        self.render_status(status_area, buf);

        Ok(())
    }
}
