use crate::{
    languages::{programming, spoken},
    models::{Lesson, LessonData},
    ui::tui::{self, screens, widgets::ScrollText, Screen},
    Error,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListState, Padding, Paragraph, StatefulWidget, Widget, Wrap},
};
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;
use tracing::info;

#[derive(Clone, Debug, Default)]
enum FocusedView {
    #[default]
    List,
    Info,
}

#[derive(Clone, Debug, Default)]
pub struct Lessons<'a> {
    /// the lesson data
    lessons: HashMap<String, LessonData>,
    /// the selected lesson
    lesson: Option<Lesson>,
    /// the lesson texts
    lesson_texts: HashMap<String, String>,
    /// the cached list
    titles: List<'a>,
    /// the list state of lesson title
    titles_state: ListState,
    /// the scrollable info window - requires lifetime for Block
    st: ScrollText<'a>,
    /// currently focused view
    focused: FocusedView,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
    /// the currently selected programming language
    programming_language: Option<programming::Code>,
}

impl Lessons<'_> {
    /// set the lessons
    async fn set_lessons(&mut self, lessons: &HashMap<String, LessonData>) -> Result<(), Error> {
        self.lessons = lessons.clone();

        if self.lessons.is_empty() {
            self.titles_state.select(None);
        } else {
            self.titles_state.select_first();
        };

        /*
        // update the cached names
        let lesson_names = self
            .get_lesson_keys()
            .iter()
            .map(|name| self.lessons.get(name).unwrap().title.clone())
            .collect::<Vec<_>>();

        // create the list of lesson titles
        self.titles = List::new(lesson_names)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::White))
            .highlight_symbol("> ");
        */

        Ok(())
    }

    async fn set_spoken_language(
        &mut self,
        spoken_language: Option<spoken::Code>,
    ) -> Result<(), Error> {
        self.spoken_language = spoken_language;
        Ok(())
    }

    async fn set_programming_language(
        &mut self,
        programming_language: Option<programming::Code>,
    ) -> Result<(), Error> {
        self.programming_language = programming_language;
        Ok(())
    }

    async fn set_selected_lesson(&mut self, lesson_key: String) -> Result<(), Error> {
        if let Some(_lesson_data) = self.lessons.get(&lesson_key) {
            /*
            // set the lesson
            self.lesson =
                Some(lesson_data.get_lesson(self.spoken_language, self.programming_language)?);

            // update the lesson texts
            self.lesson_texts = lesson_data.get_texts()?;
            */
        } else {
            info!("No lesson found for key: {}", lesson_key);
            self.lesson = None;
        }

        Ok(())
    }

    // get the selected lesson key
    fn get_selected_lesson_key(&self) -> Option<String> {
        if self.lessons.is_empty() {
            return None;
        }

        let selected_index = self.titles_state.selected().unwrap_or(0);
        self.get_lesson_keys().get(selected_index).cloned()
    }

    /// get the currently selected lesson
    fn get_selected_lesson(&self) -> Option<&Lesson> {
        self.lesson.as_ref()
    }

    /// get the sorted list of lesson keys
    fn get_lesson_keys(&self) -> Vec<String> {
        let mut lesson_keys = self.lessons.keys().cloned().collect::<Vec<_>>();
        lesson_keys.sort();
        lesson_keys
    }

    /// render the lesson list and info
    fn render_lessons(&mut self, area: Rect, buf: &mut Buffer) {
        let [lesson_titles_area, lesson_info_area] =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .areas(area);

        self.render_lesson_titles(lesson_titles_area, buf);
        self.render_lesson_info(lesson_info_area, buf);
    }

    /// render the list of lesson titles
    fn render_lesson_titles(&mut self, area: Rect, buf: &mut Buffer) {
        // figure out the titles list border fg color based on what is focused
        let fg = match self.focused {
            FocusedView::List => Color::White,
            FocusedView::Info => Color::DarkGray,
        };

        let titles = self.titles.clone().block(
            Block::default()
                .title(" lessons ")
                .padding(Padding::horizontal(1))
                .style(Style::default().fg(fg))
                .borders(Borders::ALL),
        );

        StatefulWidget::render(&titles, area, buf, &mut self.titles_state);
    }

    /// render the lesson info
    fn render_lesson_info(&mut self, area: Rect, buf: &mut Buffer) {
        let mut description = match self.get_selected_lesson() {
            Some(lesson) => lesson.description.clone(),
            None => "No lessons support the selected spoken and programming languages".to_string(),
        };

        let fg = match self.focused {
            FocusedView::List => Color::DarkGray,
            FocusedView::Info => Color::White,
        };

        let block = Block::default()
            .title(" Description ")
            .padding(Padding::horizontal(1))
            .style(Style::default().fg(fg))
            .borders(Borders::ALL);

        self.st.block(block);
        self.st
            .style(Style::default().fg(Color::White).bg(Color::Black));

        // render the scroll text
        StatefulWidget::render(&mut self.st, area, buf, &mut description);
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        // render the status bar at the bottom
        let [keys_area, langs_area] =
            Layout::horizontal([Constraint::Min(1), Constraint::Length(27)]).areas(area);

        self.render_keys(keys_area, buf);
        self.render_langs(langs_area, buf);
    }

    // render the keyboard shortcuts
    fn render_keys(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(1));

        let keys = Paragraph::new(
            "↓/↑ or j/k: scroll  |  tab: switch focus  |  enter: select  |  q: quit",
        )
        .block(block)
        .style(Style::default().fg(Color::Black).bg(Color::White))
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

        Widget::render(keys, area, buf);
    }

    // render the frames per second
    fn render_langs(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .title_alignment(Alignment::Right)
            .padding(Padding::horizontal(1));

        let spoken = match self.spoken_language {
            Some(code) => code.get_name_in_english().to_string(),
            None => "All".to_string(),
        };

        let programming = match self.programming_language {
            Some(code) => code.get_name().to_string(),
            None => "All".to_string(),
        };

        let langs = Paragraph::new(format!("[ {} | {} ]", spoken, programming))
            .block(block)
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Right);

        Widget::render(langs, area, buf);
    }

    /// handle UI events
    pub async fn handle_ui_event(
        &mut self,
        event: tui::Event,
        _to_ui: Sender<screens::Event>,
    ) -> Result<(), Error> {
        match event {
            tui::Event::SpokenLanguage(spoken_language) => {
                info!("Lessons Spoken language set: {:?}", spoken_language);
                self.set_spoken_language(spoken_language).await?;
            }
            tui::Event::ProgrammingLanguage(programming_language) => {
                info!(
                    "Lessons Programming language set: {:?}",
                    programming_language
                );
                self.set_programming_language(programming_language).await?;
            }
            // TODO: have this also pass the selected workshop for clean resuming
            tui::Event::SetLessons(lessons) => {
                info!("Setting lessons");
                self.set_lessons(&lessons).await?;
                if let Some(lesson_key) = self.get_selected_lesson_key() {
                    self.set_selected_lesson(lesson_key).await?;
                }
            }
            tui::Event::SelectLesson(lesson_key) => {
                info!("Selected lesson: {}", lesson_key);
                self.set_selected_lesson(lesson_key).await?;
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
                KeyCode::PageUp => match self.focused {
                    FocusedView::List => {
                        self.titles_state.select_first();
                        if let Some(lesson_key) = self.get_selected_lesson_key() {
                            self.set_selected_lesson(lesson_key).await?;
                        }
                    }
                    FocusedView::Info => self.st.scroll_top(),
                },
                KeyCode::PageDown => match self.focused {
                    FocusedView::List => {
                        self.titles_state.select_last();
                        if let Some(lesson_key) = self.get_selected_lesson_key() {
                            self.set_selected_lesson(lesson_key).await?;
                        }
                    }
                    FocusedView::Info => self.st.scroll_bottom(),
                },
                KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => match self.focused {
                    FocusedView::List => {
                        info!("select next");
                        self.titles_state.select_next();
                        if let Some(lesson_key) = self.get_selected_lesson_key() {
                            self.set_selected_lesson(lesson_key).await?;
                        }
                    }
                    FocusedView::Info => self.st.scroll_down(),
                },
                KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => match self.focused {
                    FocusedView::List => {
                        info!("select previous");
                        self.titles_state.select_previous();
                        info!("selected previous");
                        if let Some(lesson_key) = self.get_selected_lesson_key() {
                            info!("Setting selected workshop: {}", lesson_key);
                            self.set_selected_lesson(lesson_key).await?;
                        }
                    }
                    FocusedView::Info => self.st.scroll_up(),
                },
                KeyCode::Tab => {
                    info!("Switch focus");
                    self.focused = match self.focused {
                        FocusedView::List => FocusedView::Info,
                        FocusedView::Info => FocusedView::List,
                    };
                }
                KeyCode::Enter => {
                    if let Some(lesson_key) = self.get_selected_lesson_key() {
                        info!("Selected lesson: {}", lesson_key);
                        to_ui
                            .send(tui::Event::LoadLessons(lesson_key).into())
                            .await?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Screen for Lessons<'_> {
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

    /*
    async fn handle_event(
        &mut self,
        evt: Event,
        to_engine: Sender<Message>,
    ) -> Result<Option<UiEvent>, Error> {
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::PageUp => match self.focused {
                    FocusedView::List => self.titles_state.select_first(),
                    FocusedView::Info => self.st.scroll_top(),
                },
                KeyCode::PageDown => self.st.scroll_bottom(),
                KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => match self.focused {
                    FocusedView::List => self.titles_state.select_next(),
                    FocusedView::Info => self.st.scroll_down(),
                },
                KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => match self.focused {
                    FocusedView::List => self.titles_state.select_previous(),
                    FocusedView::Info => self.st.scroll_up(),
                },
                KeyCode::Tab => {
                    info!("Switch focus");
                    self.focused = match self.focused {
                        FocusedView::List => FocusedView::Info,
                        FocusedView::Info => FocusedView::List,
                    };
                }
                KeyCode::Enter => {
                    if let Some(lesson_key) = self.get_selected_lesson_key() {
                        info!("Lessons selected: {}", lesson_key);
                        to_engine
                            .send(Message::SetLesson {
                                name: lesson_key.clone(),
                            })
                            .await?;
                        return Ok(Some(UiEvent::SetLesson(lesson_key)));
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
        if let Message::SelectLesson {
            lessons,
            lesson_texts,
            spoken_language,
            programming_language,
        } = msg
        {
            info!("Showing select lesson screen");
            self.set_lessons(
                &lessons,
                &lesson_texts,
                spoken_language,
                programming_language,
            );
            return Ok(Some(UiEvent::SelectLesson));
        }
        Ok(None)
    }
    */

    fn render_screen(&mut self, area: Rect, buf: &mut Buffer) -> Result<(), Error> {
        // this splits the screen into a top area and a one-line bottom area
        let [lessons_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(2)])
                .flex(Flex::End)
                .areas(area);

        self.render_lessons(lessons_area, buf);
        self.render_status(status_area, buf);

        Ok(())
    }
}
