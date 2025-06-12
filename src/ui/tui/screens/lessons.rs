use crate::{
    fs,
    languages::{self, programming, spoken},
    models::{Error as ModelError, Lesson, LessonData},
    ui::tui::{self, screens, widgets::ScrollText, Screen},
    Error, Status,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::border::Set,
    text::{Line, Span},
    widgets::{block::Position, Block, Borders, List, ListState, Padding, StatefulWidget, Widget},
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::Sender;
use tracing::{debug, info, info_span, warn};

const TOP_LEFT_BORDER: Set = Set {
    top_left: "┌",
    top_right: "┐",
    bottom_left: "│",
    bottom_right: "│",
    vertical_left: "│",
    vertical_right: "│",
    horizontal_top: "─",
    horizontal_bottom: " ",
};

const TOP_BOX_BORDER: Set = Set {
    top_left: "─",
    top_right: "┐",
    bottom_left: " ",
    bottom_right: "│",
    vertical_left: " ",
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

#[derive(Clone, Debug, Default, PartialEq)]
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
    /// the title of the workshop
    workshop_title: String,
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
    async fn init<S: AsRef<str>>(
        &mut self,
        lessons: &HashMap<String, LessonData>,
        workshop_title: S,
        spoken_language: Option<spoken::Code>,
        programming_language: Option<programming::Code>,
    ) -> Result<(), Error> {
        self.lessons = lessons.clone();
        self.workshop_title = workshop_title.as_ref().to_string();
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

    // get the lesson titles with status indicators
    async fn get_titles(&mut self) -> Result<Vec<String>, Error> {
        info!("Caching lesson titles");
        self.titles_map.clear();

        // Get lessons in sorted order
        let mut lessons_with_metadata: Vec<(String, crate::models::lesson::Lesson)> = Vec::new();
        for (key, ld) in self.lessons.iter() {
            let lesson = ld.get_metadata().await?;
            lessons_with_metadata.push((key.clone(), lesson));
        }

        // Sort by lesson key (which includes ordering like 01-, 02-, etc.)
        lessons_with_metadata.sort_by(|a, b| a.0.cmp(&b.0));

        let mut titles = Vec::new();
        for (key, lesson) in lessons_with_metadata.iter() {
            let status_indicator = match lesson.status {
                crate::models::lesson::Status::Completed => "✅",
                crate::models::lesson::Status::InProgress => "⚙️",
                crate::models::lesson::Status::NotStarted => "  ",
            };

            let title_with_status = format!("{} {}", status_indicator, lesson.title);
            self.titles_map
                .insert(title_with_status.clone(), key.clone());
            titles.push(title_with_status);
        }

        Ok(titles)
    }

    // check if a lesson can be selected based on its index
    async fn can_select_lesson(&self, lesson_index: usize) -> Result<bool, Error> {
        let lesson_keys = self.get_lesson_keys();

        // First lesson can always be selected
        if lesson_index == 0 {
            return Ok(true);
        }

        // For other lessons, check if the previous lesson is completed
        if lesson_index > 0 && lesson_index < lesson_keys.len() {
            let prev_lesson_key = &lesson_keys[lesson_index - 1];
            if let Some(prev_lesson_data) = self.lessons.get(prev_lesson_key) {
                let prev_lesson = prev_lesson_data.get_metadata().await?;
                return Ok(matches!(
                    prev_lesson.status,
                    crate::models::lesson::Status::Completed
                ));
            }
        }

        Ok(false)
    }

    // check if a lesson has been completed
    async fn is_lesson_completed(&self, lesson_index: usize) -> Result<bool, Error> {
        let lesson_keys = self.get_lesson_keys();

        if lesson_index < lesson_keys.len() {
            let lesson_key = &lesson_keys[lesson_index];
            if let Some(lesson_data) = self.lessons.get(lesson_key) {
                let lesson = lesson_data.get_metadata().await?;
                return Ok(matches!(
                    lesson.status,
                    crate::models::lesson::Status::Completed
                ));
            }
        }

        Ok(false)
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

    async fn first(&mut self) -> Result<(), Error> {
        match self.focused {
            FocusedView::List => {
                if !self.lessons.is_empty() {
                    self.titles_state.select(Some(0));
                    self.cache_selected().await?;
                }
            }
            FocusedView::Info => self.st.scroll_top(),
        }
        Ok(())
    }

    async fn last(&mut self) -> Result<(), Error> {
        match self.focused {
            FocusedView::List => {
                if !self.lessons.is_empty() {
                    let last_index = self.lessons.len() - 1;
                    self.titles_state.select(Some(last_index));
                    self.cache_selected().await?;
                }
            }
            FocusedView::Info => self.st.scroll_bottom(),
        }
        Ok(())
    }

    async fn next(&mut self) -> Result<(), Error> {
        match self.focused {
            FocusedView::List => {
                if !self.lessons.is_empty() {
                    let selected_index = self.titles_state.selected().unwrap_or(0);
                    let next_index = (selected_index + 1).min(self.lessons.len() - 1);
                    self.titles_state.select(Some(next_index));
                    self.cache_selected().await?;
                }
            }
            FocusedView::Info => self.st.scroll_down(),
        }
        Ok(())
    }

    async fn prev(&mut self) -> Result<(), Error> {
        match self.focused {
            FocusedView::List => {
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
            }
            FocusedView::Info => self.st.scroll_up(),
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
            _ => Color::DarkGray,
        };

        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled("/ Select a Lesson /", Style::default().fg(fg)),
        ]);
        let titles = self.titles.clone().block(
            Block::default()
                .title(title)
                .title_style(Style::default().fg(fg))
                .padding(Padding::uniform(1))
                .style(Style::default().fg(Color::DarkGray))
                .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
                .border_set(TOP_LEFT_BORDER),
        );

        StatefulWidget::render(&titles, area, buf, &mut self.titles_state);
    }

    /// render the lesson info
    fn render_lesson_info(&mut self, area: Rect, buf: &mut Buffer) {
        let fg = if self.focused == FocusedView::Info {
            Color::White
        } else {
            Color::DarkGray
        };

        let mut description = match &self.selected {
            Some(lesson) => lesson.description.clone(),
            None => "No lessons support the selected spoken and programming languages".to_string(),
        };

        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled("/ Description /", Style::default().fg(fg)),
        ]);
        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(fg))
            .padding(Padding::top(1))
            .style(Style::default().fg(Color::DarkGray))
            .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
            .border_set(TOP_BOX_BORDER);

        self.st.block(block);
        self.st.style(Style::default().fg(Color::White));

        // render the scroll text
        StatefulWidget::render(&mut self.st, area, buf, &mut description);
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
                "/ j,k scroll / ⇥ focus / ↵ select / b back / q quit /",
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

    // render the frames per second
    fn render_langs(&mut self, area: Rect, buf: &mut Buffer) {
        let spoken = languages::spoken_name(self.spoken_language);
        let programming = languages::programming_name(self.programming_language);
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
            tui::Event::LoadLessons => {
                let span = info_span!("Lessons");
                let _enter = span.enter();
                let (spoken, programming, workshop) = {
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
                    )
                };
                if let Some(workshop_data) = fs::workshops::load(&workshop) {
                    info!(
                        "Loading lessons for workshop: {} (spoken: {:?}, programming: {:?})",
                        &workshop,
                        languages::spoken_name(spoken),
                        languages::programming_name(programming),
                    );
                    let lessons = workshop_data.get_lessons_data(spoken, programming).await?;
                    let workshop_title = workshop_data.get_metadata(spoken).await?.title;
                    self.init(&lessons, workshop_title, spoken, programming)
                        .await?;
                    to_ui
                        .send((None, tui::Event::Show(screens::Screens::Lessons)).into())
                        .await?;
                } else {
                    warn!("Failed to load workshop data for: {}", &workshop);
                }
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
                KeyCode::PageUp => self.first().await?,
                KeyCode::PageDown => self.last().await?,
                KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => self.next().await?,
                KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => self.prev().await?,
                KeyCode::Char('b') | KeyCode::Esc => {
                    to_ui
                        .send((None, tui::Event::SetWorkshop(None, HashMap::default())).into())
                        .await?;
                }
                KeyCode::Tab => {
                    self.focused = match self.focused {
                        FocusedView::List => FocusedView::Info,
                        FocusedView::Info => FocusedView::List,
                    };
                }
                KeyCode::Enter => {
                    if let Some(selected_index) = self.titles_state.selected() {
                        // Check if the lesson can be selected and is not completed
                        let can_select = self.can_select_lesson(selected_index).await?;
                        let is_completed = self.is_lesson_completed(selected_index).await?;

                        if can_select && !is_completed {
                            to_ui
                                .send(
                                    (None, tui::Event::SetLesson(self.get_selected_lesson_key()))
                                        .into(),
                                )
                                .await?;
                        }
                        // If lesson cannot be selected or is completed, do nothing (ignore the input)
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
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(area);

        self.render_lessons(lessons_area, buf);
        self.render_status(status_area, buf);

        Ok(())
    }
}
