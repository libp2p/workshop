use crate::{
    fs,
    languages::{programming, spoken},
    models::{Lesson, LessonData},
    ui::tui::{self, screens, widgets::ScrollText, Screen},
    Error, Status,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListState, Padding, Paragraph, StatefulWidget, Widget, Wrap},
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};
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
    /// the cached selected lesson data
    selected: Option<Lesson>,
    /// the map of lesson titles to lesson keys
    titles_map: BTreeMap<String, String>,
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
    async fn init(
        &mut self,
        lessons: &HashMap<String, LessonData>,
        spoken_language: Option<spoken::Code>,
        programming_language: Option<programming::Code>,
    ) -> Result<(), Error> {
        self.lessons = lessons.clone();
        self.spoken_language = spoken_language;
        self.programming_language = programming_language;

        if self.lessons.is_empty() {
            self.titles_state.select(None);
        } else {
            self.titles_state.select_first();
        };

        // get the list of titles
        let titles = self.get_titles().await?;
        self.titles = List::new(titles)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::White))
            .highlight_symbol("> ");

        // cache all of the data for the selected lesson
        self.cache_selected().await?;

        Ok(())
    }

    // get the lesson titles
    async fn get_titles(&mut self) -> Result<Vec<String>, Error> {
        info!("Caching lesson titles");
        self.titles_map.clear();
        for (key, ld) in self.lessons.iter() {
            let lesson = ld.get_metadata().await?;
            self.titles_map.insert(lesson.title.clone(), key.clone());
        }
        Ok(self.titles_map.keys().cloned().collect())
    }

    // cached selected lesson data
    async fn cache_selected(&mut self) -> Result<(), Error> {
        info!("Caching selected lesson data");
        self.selected = None;
        if let Some(lesson_key) = self.get_selected_lesson_key() {
            if let Some(lesson_data) = self.lessons.get(&lesson_key) {
                let lesson = lesson_data.get_metadata().await?;
                self.selected = Some(lesson);
            }
        }
        Ok(())
    }

    // select first lesson
    async fn select_first(&mut self) -> Result<(), Error> {
        if !self.lessons.is_empty() {
            self.titles_state.select(Some(0));
            self.cache_selected().await?;
        }
        Ok(())
    }

    // select last lesson
    async fn select_last(&mut self) -> Result<(), Error> {
        if !self.lessons.is_empty() {
            let last_index = self.lessons.len() - 1;
            self.titles_state.select(Some(last_index));
            self.cache_selected().await?;
        }
        Ok(())
    }

    // select next lesson
    async fn select_next(&mut self) -> Result<(), Error> {
        if !self.lessons.is_empty() {
            let selected_index = self.titles_state.selected().unwrap_or(0);
            let next_index = (selected_index + 1).min(self.lessons.len() - 1);
            self.titles_state.select(Some(next_index));
            self.cache_selected().await?;
        }
        Ok(())
    }

    // select previous lesson
    async fn select_prev(&mut self) -> Result<(), Error> {
        if !self.lessons.is_empty() {
            let selected_index = self.titles_state.selected().unwrap_or(0);
            let prev_index = if selected_index > 0 {
                selected_index - 1
            } else {
                0
            };
            self.titles_state.select(Some(prev_index));
            self.cache_selected().await?;
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

    // get the sorted list of lesson keys
    fn get_lesson_keys(&self) -> Vec<String> {
        self.titles_map.values().cloned().collect()
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
        let mut description = match &self.selected {
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
        to_ui: Sender<screens::Event>,
        status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        match event {
            tui::Event::LoadLessons => {
                info!("Loading lessons");
                let (spoken, programming, workshop) = {
                    let status = status
                        .lock()
                        .map_err(|e| Error::StatusLock(e.to_string()))?;
                    (
                        status.spoken_language(),
                        status.programming_language(),
                        status.workshop().unwrap(),
                    )
                };
                if let Some(workshop_data) = fs::workshops::load(&workshop) {
                    info!("Loading lessons for workshop: {}", &workshop);
                    let lessons = workshop_data.get_lessons_data(spoken, programming).await?;
                    self.init(&lessons, spoken, programming).await?;
                    to_ui
                        .send((None, tui::Event::Show(screens::Screens::Lessons)).into())
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
                KeyCode::PageUp => match self.focused {
                    FocusedView::List => {
                        self.select_first().await?;
                    }
                    FocusedView::Info => self.st.scroll_top(),
                },
                KeyCode::PageDown => match self.focused {
                    FocusedView::List => {
                        self.select_last().await?;
                    }
                    FocusedView::Info => self.st.scroll_bottom(),
                },
                KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => match self.focused {
                    FocusedView::List => {
                        self.select_next().await?;
                    }
                    FocusedView::Info => self.st.scroll_down(),
                },
                KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => match self.focused {
                    FocusedView::List => {
                        self.select_prev().await?;
                    }
                    FocusedView::Info => self.st.scroll_up(),
                },
                KeyCode::Char('b') | KeyCode::Esc => {
                    to_ui
                        .send((Some(screens::Screens::Workshops), tui::Event::LoadWorkshops).into())
                        .await?;
                }
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
                            .send((None, tui::Event::SetLesson(lesson_key)).into())
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
        let [lessons_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(2)])
                .flex(Flex::End)
                .areas(area);

        self.render_lessons(lessons_area, buf);
        self.render_status(status_area, buf);

        Ok(())
    }
}
