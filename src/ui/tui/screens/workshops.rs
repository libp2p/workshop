use crate::{
    evt, fs,
    languages::{self, programming, spoken},
    models::{workshop, Workshop, WorkshopData},
    ui::tui::{
        self,
        screens::{self, Screens},
        widgets::{LessonBox, LessonBoxState, ScrollBox},
        Screen,
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
    widgets::{block::Position, Block, Borders, List, ListState, Padding, StatefulWidget, Widget},
};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, info_span};

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

const BOTTOM_BOX_BORDER: Set = Set {
    top_left: "─",
    top_right: "┤",
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

#[derive(Clone, Debug)]
enum FocusedView<'a> {
    List(List<'a>, ListState),
    Metadata(ScrollBox<'a>),
    Description(LessonBox<'a>, LessonBoxState),
    SetupInstructions(LessonBox<'a>, LessonBoxState),
}

impl Default for FocusedView<'_> {
    fn default() -> Self {
        FocusedView::List(List::default(), ListState::default())
    }
}

impl FocusedView<'_> {
    pub fn as_str(&self) -> &'static str {
        match self {
            FocusedView::List(..) => "list",
            FocusedView::Metadata(..) => "metadata",
            FocusedView::Description(..) => "description",
            FocusedView::SetupInstructions(..) => "setup",
        }
    }
}

impl fmt::Display for FocusedView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FocusedView::List(..) => write!(f, "List"),
            FocusedView::Metadata(..) => write!(f, "Metadata"),
            FocusedView::Description(..) => write!(f, "Description"),
            FocusedView::SetupInstructions(..) => write!(f, "Setup Instructions"),
        }
    }
}

impl Widget for &mut FocusedView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            FocusedView::List(ref list, ref mut state) => {
                StatefulWidget::render(list, area, buf, state);
            }
            FocusedView::Metadata(ref mut scroll_box) => {
                Widget::render(&mut *scroll_box, area, buf);
            }
            FocusedView::Description(ref lesson_box, ref mut state) => {
                StatefulWidget::render(lesson_box.clone(), area, buf, state);
            }
            FocusedView::SetupInstructions(ref lesson_box, ref mut state) => {
                StatefulWidget::render(lesson_box.clone(), area, buf, state);
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Cached {
    workshop: Workshop,
    license: String,
}

#[derive(Clone, Debug, Default)]
pub struct Workshops<'a> {
    /// the list of workshops
    workshops: HashMap<String, WorkshopData>,
    /// the currently selected workshop data
    selected: Option<Cached>,
    /// the map of workshop titles to workshop keys in sorted order
    titles_map: BTreeMap<String, String>,
    /// the views
    views: HashMap<&'static str, FocusedView<'a>>,
    /// currently focused view
    focused: &'static str,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
    /// the currently selected programming language
    programming_language: Option<programming::Code>,
}

impl Workshops<'_> {
    /// create a new Workshops instance
    pub fn new() -> Self {
        Workshops {
            views: [
                (
                    "list",
                    FocusedView::List(List::default(), ListState::default()),
                ),
                ("metadata", FocusedView::Metadata(ScrollBox::default())),
                (
                    "description",
                    FocusedView::Description(LessonBox::default(), LessonBoxState::default()),
                ),
                (
                    "setup",
                    FocusedView::SetupInstructions(LessonBox::default(), LessonBoxState::default()),
                ),
            ]
            .into_iter()
            .collect(),
            focused: FocusedView::default().as_str(),
            ..Default::default()
        }
    }

    /// set the workshops
    async fn init(
        &mut self,
        workshops: &HashMap<String, WorkshopData>,
        spoken_language: Option<spoken::Code>,
        programming_language: Option<programming::Code>,
    ) -> Result<(), Error> {
        self.workshops = workshops.clone();
        self.spoken_language = spoken_language;
        self.programming_language = programming_language;

        // get the workshop titles
        let t = self.get_titles().await?;

        if let Some(FocusedView::List(titles, state)) = self.views.get_mut("list") {
            // set the initial focus
            if self.workshops.is_empty() {
                state.select(None);
            } else {
                state.select_first();
            }

            // set the titles
            *titles = List::new(t)
                .highlight_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .style(Style::default().fg(Color::White))
                .highlight_symbol("> ");
        }

        // cache all of the data for the selected workshop
        self.cache_selected().await?;

        Ok(())
    }

    // get the workshop titles with status indicators
    async fn get_titles(&mut self) -> Result<Vec<String>, Error> {
        debug!("Caching workshop titles");
        self.titles_map.clear();

        // Get workshops with their calculated status
        let mut workshops_with_status: Vec<(String, String, workshop::Status)> = Vec::new();
        for (key, wd) in self.workshops.iter() {
            let workshop = wd.get_metadata(self.spoken_language).await?;
            let status = workshop.status.clone();
            workshops_with_status.push((key.clone(), workshop.title.clone(), status));
        }

        // Sort by workshop title
        workshops_with_status.sort_by(|a, b| a.1.cmp(&b.1));

        for (key, title, status) in workshops_with_status.iter() {
            let status_indicator = match status {
                workshop::Status::Completed => "✅ ",
                workshop::Status::InProgress => "🤔 ",
                workshop::Status::NotStarted => "   ",
            };

            let title_with_status = format!("{status_indicator} {title}");
            self.titles_map
                .insert(title_with_status.clone(), key.clone());
        }

        Ok(self.titles_map.keys().cloned().collect())
    }

    // cached selected workshop data
    async fn cache_selected(&mut self) -> Result<(), Error> {
        debug!("Caching selected workshop data");
        self.selected = None;
        if let Some(workshop_key) = self.get_selected_workshop_key() {
            if let Some(workshop_data) = self.workshops.get(&workshop_key) {
                let workshop = workshop_data.get_metadata(self.spoken_language).await?;
                let languages = workshop_data.get_all_languages().clone();
                let description = workshop_data
                    .get_description(self.spoken_language)
                    .await
                    .unwrap_or_default();
                let setup_instructions = workshop_data
                    .get_setup_instructions(self.spoken_language, self.programming_language)
                    .await
                    .unwrap_or_default();
                let license = workshop_data.get_license().await?;

                // update the scroll boxes
                let metadata = format!(
                    "Status: {}\nAuthors: {}\nCopyright: {}\nLicense: {}\nHomepage: {}\nDifficulty: {}\nLanguages:\n{}",
                    workshop.status,
                    workshop
                        .authors
                        .iter()
                        .map(|a| format!(" - {a}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    workshop.copyright,
                    workshop.license,
                    workshop.homepage,
                    workshop.difficulty,
                    languages
                        .iter()
                        .map(|(spoken_lang, programming_langs)| {
                            format!(
                                " - {}: {}",
                                spoken_lang.get_name_in_native(),
                                programming_langs
                                    .iter()
                                    .map(|pl| pl.get_name())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                );

                for (_, v) in self.views.iter_mut() {
                    match v {
                        FocusedView::Metadata(scroll_box) => scroll_box.set_text(&metadata),
                        FocusedView::Description(_, state) => {
                            *state = LessonBoxState::from_markdown(&description);
                        }
                        FocusedView::SetupInstructions(_, state) => {
                            *state = LessonBoxState::from_markdown(&setup_instructions);
                        }
                        _ => {}
                    }
                }

                self.selected = Some(Cached { workshop, license });

                return Ok(());
            }
        }

        // set the boxes to default text
        for (_, v) in self.views.iter_mut() {
            match v {
                FocusedView::Metadata(ref mut scroll_box) => {
                    scroll_box.set_text("No metadata available for the selected workshop");
                }
                FocusedView::Description(_, ref mut state) => {
                    *state = LessonBoxState::from_markdown("");
                }
                FocusedView::SetupInstructions(_, ref mut state) => {
                    *state = LessonBoxState::from_markdown("");
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn first(&mut self) -> Result<(), Error> {
        if let Some(v) = self.views.get_mut(self.focused) {
            match v {
                FocusedView::List(_, state) => {
                    if !self.workshops.is_empty() {
                        state.select(Some(0));
                        self.cache_selected().await?;
                    }
                }
                FocusedView::Metadata(scroll_box) => {
                    scroll_box.scroll_top();
                }
                FocusedView::Description(_, state) => {
                    state.scroll_top();
                }
                FocusedView::SetupInstructions(_, state) => {
                    state.scroll_top();
                }
            }
        }
        Ok(())
    }

    async fn last(&mut self) -> Result<(), Error> {
        if let Some(v) = self.views.get_mut(self.focused) {
            match v {
                FocusedView::List(_, state) => {
                    let last_index = self.workshops.len() - 1;
                    state.select(Some(last_index));
                    self.cache_selected().await?;
                }
                FocusedView::Metadata(scroll_box) => {
                    scroll_box.scroll_bottom();
                }
                FocusedView::Description(_, state) => {
                    state.scroll_bottom();
                }
                FocusedView::SetupInstructions(_, state) => {
                    state.scroll_bottom();
                }
            }
        }
        Ok(())
    }

    async fn next(&mut self) -> Result<(), Error> {
        if let Some(v) = self.views.get_mut(self.focused) {
            match v {
                FocusedView::List(_, state) => {
                    if !self.workshops.is_empty() {
                        let selected_index = state.selected().unwrap_or(0);
                        let next_index = (selected_index + 1).min(self.workshops.len() - 1);
                        state.select(Some(next_index));
                        self.cache_selected().await?;
                    }
                }
                FocusedView::Metadata(scroll_box) => {
                    scroll_box.scroll_down();
                }
                FocusedView::Description(_, state) => {
                    state.scroll_down();
                }
                FocusedView::SetupInstructions(_, state) => {
                    state.scroll_down();
                }
            }
        }
        Ok(())
    }

    async fn prev(&mut self) -> Result<(), Error> {
        if let Some(v) = self.views.get_mut(self.focused) {
            match v {
                FocusedView::List(_, state) => {
                    if !self.workshops.is_empty() {
                        let selected_index = state.selected().unwrap_or(0);
                        let prev_index = if selected_index > 0 {
                            selected_index - 1
                        } else {
                            0
                        };
                        state.select(Some(prev_index));
                        self.cache_selected().await?;
                    }
                }
                FocusedView::Metadata(scroll_box) => {
                    scroll_box.scroll_up();
                }
                FocusedView::Description(_, state) => {
                    state.scroll_up();
                }
                FocusedView::SetupInstructions(_, state) => {
                    state.scroll_up();
                }
            }
        }
        Ok(())
    }

    // get the selected workshop key
    fn get_selected_workshop_key(&self) -> Option<String> {
        if self.workshops.is_empty() {
            return None;
        }
        if let Some(FocusedView::List(_, state)) = self.views.get(self.focused) {
            let selected_index = state.selected().unwrap_or(0);
            self.get_workshop_keys().get(selected_index).cloned()
        } else {
            None
        }
    }

    // get the sorted list of workshop keys
    fn get_workshop_keys(&self) -> Vec<String> {
        self.titles_map.values().cloned().collect()
    }

    // get the cached URL for the selected workshop
    fn get_url(&self) -> Option<String> {
        if let Some(Cached { workshop, .. }) = &self.selected {
            Some(workshop.homepage.clone())
        } else {
            None
        }
    }

    // get the cached license text for the selected workshop
    fn get_license(&self) -> Option<String> {
        if let Some(Cached { license, .. }) = &self.selected {
            Some(license.clone())
        } else {
            None
        }
    }

    /// render the workshop list and info
    fn render_workshops(&mut self, area: Rect, buf: &mut Buffer) {
        let [workshop_titles_area, workshop_info_area] =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .areas(area);

        self.render_workshop_titles(workshop_titles_area, buf);
        self.render_workshop_info(workshop_info_area, buf);
    }

    /// render the list of workshop titles
    fn render_workshop_titles(&mut self, area: Rect, buf: &mut Buffer) {
        // figure out the titles list border fg color based on what is focused
        let fg = if self.focused == "list" {
            Color::White
        } else {
            Color::DarkGray
        };

        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled("/ Select a Workshop /", Style::default().fg(fg)),
        ]);

        if let Some(view) = self.views.get_mut("list") {
            if let FocusedView::List(list, _) = view {
                *list = list.clone().block(
                    Block::default()
                        .title(title)
                        .padding(Padding::uniform(1))
                        .style(Style::default().fg(fg))
                        .border_style(Style::default().fg(Color::DarkGray))
                        .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
                        .border_set(TOP_LEFT_BORDER),
                );
            }

            Widget::render(view, area, buf);
        };
    }

    /// render the workshop info
    fn render_workshop_info(&mut self, area: Rect, buf: &mut Buffer) {
        let areas: [Rect; 3] = Layout::vertical([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .flex(Flex::End)
        .areas(area);

        self.render_workshop_box(areas[0], buf, "metadata", TOP_BOX_BORDER);
        self.render_workshop_box(areas[1], buf, "description", BOTTOM_BOX_BORDER);
        self.render_workshop_box(areas[2], buf, "setup", BOTTOM_BOX_BORDER);
    }

    // render the workshop box
    fn render_workshop_box(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        view: &'static str,
        border_set: Set,
    ) {
        // figure out the box border fg color based on what is focused
        let fg = if self.focused == view {
            Color::White
        } else {
            Color::DarkGray
        };

        if let Some(view) = self.views.get_mut(view) {
            // get the box title
            let title = Line::from(vec![
                Span::styled("─", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("/ {view} /"), Style::default().fg(fg)),
            ]);

            // set the block
            match view {
                FocusedView::Metadata(widget) => {
                    widget.block(
                        Block::default()
                            .title(title)
                            .padding(Padding::uniform(1))
                            .style(Style::default().fg(fg))
                            .border_style(Style::default().fg(Color::DarkGray))
                            .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
                            .border_set(border_set),
                    );
                }
                FocusedView::Description(widget, _) | FocusedView::SetupInstructions(widget, _) => {
                    *widget = widget.clone().block(
                        Block::default()
                            .title(title)
                            .padding(Padding::uniform(1))
                            .style(Style::default().fg(fg))
                            .border_style(Style::default().fg(Color::DarkGray))
                            .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
                            .border_set(border_set),
                    );
                }
                _ => return,
            };
            Widget::render(view, area, buf);
        };
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        // render the status bar at the bottom
        let [keys_area, lang_area] =
            Layout::horizontal([Constraint::Min(1), Constraint::Length(27)]).areas(area);

        self.render_keys(keys_area, buf);
        self.render_lang(lang_area, buf);
    }

    // render the keyboard shortcuts
    fn render_keys(&mut self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "/ j,k scroll / ⇥ focus / ↵ select / w homepage / l license / f filter / q quit /",
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
    fn render_lang(&mut self, area: Rect, buf: &mut Buffer) {
        let spoken = languages::spoken_name(self.spoken_language);
        let programming = languages::programming_name(self.programming_language);
        let title = Line::from(vec![
            Span::styled(
                format!("/ {spoken} / {programming} /"),
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
            tui::Event::LoadWorkshops => {
                let span = info_span!("Workshops");
                let _enter = span.enter();
                let (spoken, programming) = {
                    let status = status.lock().unwrap();
                    (status.spoken_language(), status.programming_language())
                };
                info!(
                    "Loading workshops (spoken: {:?}, programming: {:?})",
                    languages::spoken_name(spoken),
                    languages::programming_name(programming),
                );
                let workshops = fs::application::all_workshops_filtered(spoken, programming)?;
                self.init(&workshops, spoken, programming).await?;
                to_ui
                    .send((None, tui::Event::Show(screens::Screens::Workshops)).into())
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
        status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        if let event::Event::Key(key) = event {
            match key.code {
                KeyCode::PageUp => self.first().await?,
                KeyCode::PageDown => self.last().await?,
                KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => self.next().await?,
                KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => self.prev().await?,
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    if let Some(license) = self.get_license() {
                        to_ui
                            .send(
                                (
                                    Some(screens::Screens::License),
                                    tui::Event::ShowLicense(license),
                                )
                                    .into(),
                            )
                            .await?;
                    } else {
                        debug!("No selected workshop");
                    }
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    // we're filtering workshops based on spoken and programming languages
                    // clear out the local status spoken and programming languages so we can
                    // set them from all valid selections
                    {
                        let mut status = status
                            .lock()
                            .map_err(|e| Error::StatusLock(e.to_string()))?;
                        status.set_spoken_language(None, false);
                        status.set_programming_language(None, false);
                    }
                    let all_languages = fs::application::get_all_languages()?;
                    let set_workshop = evt!(Screens::Workshops, tui::Event::LoadWorkshops);
                    let change_programming_language = evt!(
                        Screens::Programming,
                        tui::Event::ChangeProgrammingLanguage(
                            all_languages.clone(),
                            None,
                            true,
                            Some(set_workshop)
                        ),
                    );
                    let change_spoken_language = evt!(
                        Screens::Spoken,
                        tui::Event::ChangeSpokenLanguage(
                            all_languages.clone(),
                            None,
                            true,
                            Some(change_programming_language),
                        ),
                    );
                    to_ui.send(change_spoken_language.into()).await?;
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    if let Some(url) = self.get_url() {
                        info!("Open homepage: {}", url);
                        if let Err(e) = webbrowser::open(&url) {
                            error!("Failed to open browser: {}", e);
                        }
                    }
                }
                KeyCode::Tab => {
                    if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                        // switch focus to the previous view
                        self.focused = match self.focused {
                            "list" => "setup",
                            "metadata" => "list",
                            "description" => "metadata",
                            "setup" => "description",
                            &_ => "list",
                        };
                    } else {
                        // switch focus to the next view
                        self.focused = match self.focused {
                            "list" => "metadata",
                            "metadata" => "description",
                            "description" => "setup",
                            "setup" => "list",
                            &_ => "list",
                        };
                    }
                }
                KeyCode::Enter => {
                    // we're choosing a workshop so clear out the local status spoken and
                    // programming languages so we set them from the valid selections associated
                    // with the selected workshop
                    {
                        let mut status = status
                            .lock()
                            .map_err(|e| Error::StatusLock(e.to_string()))?;
                        status.set_spoken_language(None, false);
                        status.set_programming_language(None, false);
                    }
                    if let Some(workshop_key) = self.get_selected_workshop_key() {
                        if let Some(workshop_data) = self.workshops.get(&workshop_key) {
                            let all_languages = workshop_data.get_all_languages().clone();
                            to_ui
                                .send(
                                    (
                                        None,
                                        tui::Event::SetWorkshop(
                                            self.get_selected_workshop_key(),
                                            all_languages,
                                        ),
                                    )
                                        .into(),
                                )
                                .await?;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Screen for Workshops<'_> {
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
        let [workshops_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(area);

        self.render_workshops(workshops_area, buf);
        self.render_status(status_area, buf);

        Ok(())
    }
}
