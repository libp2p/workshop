use crate::{
    command::CommandResult,
    evt, fs,
    languages::{programming, spoken},
    models::{lesson, workshop, Error as ModelError, LessonData},
    ui::tui::{
        self,
        screens::{self, Screens},
        widgets::{LessonBox, LessonBoxState},
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
    widgets::{block::Position, Block, Borders, Padding, StatefulWidget, Widget},
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use tracing::{debug, info};

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
pub struct Lesson {
    /// the title of the workshop
    workshop_title: String,
    /// the title of the lesson
    lesson_title: String,
    /// the lesson box state for rendering markdown content
    lesson_state: LessonBoxState,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
    /// the currently selected programming language
    programming_language: Option<programming::Code>,
}

impl Lesson {
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
        self.lesson_state = LessonBoxState::from_markdown(text.as_ref());
        self.spoken_language = spoken_language;
        self.programming_language = programming_language;
        Ok(())
    }

    /// check if all lessons in the workshop are completed
    async fn check_all_lessons_completed(
        &self,
        lessons: &std::collections::HashMap<String, LessonData>,
    ) -> Result<bool, Error> {
        for lesson_data in lessons.values() {
            let lesson = lesson_data.get_metadata().await?;
            if !matches!(lesson.status, lesson::Status::Completed) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// render the lesson
    fn render_lesson(&mut self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("/ {} /", self.lesson_title),
                Style::default().fg(Color::White),
            ),
        ]);
        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(Color::White))
            .padding(Padding::uniform(1))
            .style(Style::default().fg(Color::DarkGray))
            .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
            .border_set(TOP_BORDER);

        let lesson_widget = LessonBox::new()
            .block(block)
            .style(Style::default().fg(Color::White));

        // render the lesson box
        StatefulWidget::render(lesson_widget, area, buf, &mut self.lesson_state);
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
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "/ j,k scroll / ⇥ next hint / ↵ expand hint / c check / b back / q quit /",
                Style::default().fg(Color::White),
            ),
        ]);
        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(Color::White))
            .title_position(Position::Bottom)
            .title_alignment(Alignment::Left)
            .style(Style::default().fg(Color::DarkGray))
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
                Style::default().fg(Color::White),
            ),
            Span::styled("─", Style::default().fg(Color::DarkGray)),
        ]);

        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(Color::White))
            .title_position(Position::Bottom)
            .title_alignment(Alignment::Right)
            .style(Style::default().fg(Color::DarkGray))
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
                debug!("Loading lessons");
                let (spoken, programming, workshop, lesson) = {
                    let status = status
                        .lock()
                        .map_err(|e| Error::StatusLock(e.to_string()))?;
                    (
                        status.spoken_language(),
                        status.programming_language(),
                        status
                            .workshop()
                            .map(String::from)
                            .ok_or(ModelError::NoWorkshopSpecified)?,
                        status
                            .lesson()
                            .map(String::from)
                            .ok_or(ModelError::NoLessonSpecified)?,
                    )
                };
                if let Some(workshop_data) = fs::workshops::load(&workshop) {
                    debug!("Loading lessons for workshop: {}", &workshop);
                    let lessons = workshop_data.get_lessons_data(spoken, programming).await?;
                    let workshop_title = workshop_data.get_metadata(spoken).await?.title;
                    let lesson_data = lessons
                        .get(&lesson)
                        .ok_or(ModelError::NoLessonData(lesson.to_string()))?;
                    let lesson_text = lesson_data.get_text().await?;
                    let lesson_metadata = lesson_data.get_metadata().await?;
                    let lesson_title = lesson_metadata.title.clone();

                    // Set lesson status to InProgress if it's NotStarted
                    if matches!(lesson_metadata.status, lesson::Status::NotStarted) {
                        lesson_data
                            .update_status(lesson::Status::InProgress)
                            .await?;
                        debug!("Updated lesson status to InProgress: {}", lesson_title);
                    }

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
            tui::Event::SolutionComplete => {
                // Set the lesson status to completed
                let (spoken, programming, workshop, lesson) = {
                    let status = status
                        .lock()
                        .map_err(|e| Error::StatusLock(e.to_string()))?;
                    (
                        status.spoken_language(),
                        status.programming_language(),
                        status
                            .workshop()
                            .map(String::from)
                            .ok_or(ModelError::NoWorkshopSpecified)?,
                        status
                            .lesson()
                            .map(String::from)
                            .ok_or(ModelError::NoLessonSpecified)?,
                    )
                };

                if let Some(workshop_data) = fs::workshops::load(&workshop) {
                    let lessons = workshop_data.get_lessons_data(spoken, programming).await?;
                    if let Some(lesson_data) = lessons.get(&lesson) {
                        lesson_data.update_status(lesson::Status::Completed).await?;
                        debug!("Updated lesson status to Completed: {}", lesson);

                        // Check if all lessons are completed
                        let all_completed = self.check_all_lessons_completed(&lessons).await?;

                        if all_completed {
                            // Set the workshop as complete
                            workshop_data
                                .update_status(spoken, workshop::Status::Completed)
                                .await?;
                            // Return to workshops screen if all lessons are completed
                            let set_workshop = evt!(
                                None,
                                tui::Event::SetWorkshop(None, std::collections::HashMap::default())
                            );
                            let hide_log = evt!(None, tui::Event::HideLog(Some(set_workshop)));
                            let workshop_complete = evt!(
                                Screens::Log,
                                tui::Event::CommandCompleted(
                                    CommandResult {
                                        success: true,
                                        exit_code: 0,
                                        last_line: "All lessons completed!".to_string()
                                    },
                                    Some(hide_log),
                                    None
                                )
                            );
                            to_ui.send(workshop_complete.into()).await?;
                        } else {
                            // Return to lessons screen to show updated status
                            let load_lessons = evt!(Screens::Lessons, tui::Event::LoadLessons);
                            let hide_log = evt!(None, tui::Event::HideLog(Some(load_lessons)));
                            to_ui.send(hide_log.into()).await?;
                        }
                    }
                }
            }
            tui::Event::SolutionIncomplete => {
                let load_lesson = evt!(Screens::Lesson, tui::Event::LoadLesson);
                let hide_log = evt!(None, tui::Event::HideLog(Some(load_lesson)));
                to_ui.send(hide_log.into()).await?;
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
                KeyCode::PageUp => self.lesson_state.scroll_top(),
                KeyCode::PageDown => self.lesson_state.scroll_bottom(),
                KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => {
                    self.lesson_state.highlight_down()
                }
                KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => {
                    self.lesson_state.highlight_up()
                }
                KeyCode::Tab => {
                    // Tab key for hint navigation - could be expanded later
                }
                KeyCode::Enter => {
                    // Toggle hint if highlighted line is a hint title
                    self.lesson_state.toggle_highlighted_hint(80); // Default width, could be dynamic
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    // Check solution
                    let success = evt!(Screens::Lesson, tui::Event::SolutionComplete);
                    let failure = evt!(Screens::Lesson, tui::Event::SolutionIncomplete);
                    let check_solution = evt!(
                        None,
                        tui::Event::CheckSolution(Some(success), Some(failure)),
                    );
                    let show_log = evt!(None, tui::Event::ShowLog(Some(check_solution)));
                    to_ui.send(show_log.into()).await?;
                }
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
impl Screen for Lesson {
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
