use crate::{
    evt, fs,
    languages::{self, programming, spoken},
    models::{Workshop, WorkshopData},
    ui::tui::{
        self,
        screens::{self, Screens},
        widgets::ScrollBox,
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

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq)]
enum FocusedView {
    #[default]
    List,
    Metadata,
    Description,
    SetupInstructions,
}

impl fmt::Display for FocusedView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FocusedView::List => write!(f, "List"),
            FocusedView::Metadata => write!(f, "Metadata"),
            FocusedView::Description => write!(f, "Description"),
            FocusedView::SetupInstructions => write!(f, "Setup Instructions"),
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
    /// the cached list
    titles: List<'a>,
    /// the list state of workshop title
    titles_state: ListState,
    /// workshop data boxes
    boxes: HashMap<FocusedView, ScrollBox<'a>>,
    /// currently focused view
    focused: FocusedView,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
    /// the currently selected programming language
    programming_language: Option<programming::Code>,
}

impl Workshops<'_> {
    /// create a new Workshops instance
    pub fn new() -> Self {
        Workshops {
            boxes: [
                (FocusedView::Metadata, ScrollBox::default()),
                (FocusedView::Description, ScrollBox::default()),
                (FocusedView::SetupInstructions, ScrollBox::default()),
            ]
            .into_iter()
            .collect(),
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

        if self.workshops.is_empty() {
            self.titles_state.select(None);
        } else {
            self.titles_state.select_first();
        };

        // create the titles list
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

        // cache all of the data for the selected workshop
        self.cache_selected().await?;

        Ok(())
    }

    // get the workshop titles
    async fn get_titles(&mut self) -> Result<Vec<String>, Error> {
        debug!("Caching workshop titles");
        self.titles_map.clear();
        for (key, wd) in self.workshops.iter() {
            let workshop = wd.get_metadata(self.spoken_language).await?;
            self.titles_map.insert(workshop.title.clone(), key.clone());
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
                    "Authors: {}\nCopyright: {}\nLicense: {}\nHomepage: {}\nDifficulty: {}\nLanguages:\n{}",
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

                for (v, b) in self.boxes.iter_mut() {
                    match v {
                        FocusedView::Metadata => b.set_text(&metadata),
                        FocusedView::Description => b.set_text(&description),
                        FocusedView::SetupInstructions => b.set_text(&setup_instructions),
                        _ => {}
                    }
                }

                self.selected = Some(Cached { workshop, license });

                return Ok(());
            }
        }

        // set the boxes to default text
        for (v, b) in self.boxes.iter_mut() {
            match v {
                FocusedView::Metadata => {
                    b.set_text("No workshops support the selected spoken and programming languages")
                }
                FocusedView::Description => b.set_text(""),
                FocusedView::SetupInstructions => b.set_text(""),
                _ => {}
            }
        }

        Ok(())
    }

    async fn first(&mut self) -> Result<(), Error> {
        match &self.focused {
            FocusedView::List => {
                if !self.workshops.is_empty() {
                    self.titles_state.select(Some(0));
                    self.cache_selected().await?;
                }
            }
            view => {
                if let Some(sb) = self.boxes.get_mut(view) {
                    sb.scroll_top();
                }
            }
        }
        Ok(())
    }

    async fn last(&mut self) -> Result<(), Error> {
        match &self.focused {
            FocusedView::List => {
                if !self.workshops.is_empty() {
                    let last_index = self.workshops.len() - 1;
                    self.titles_state.select(Some(last_index));
                    self.cache_selected().await?;
                }
            }
            view => {
                if let Some(sb) = self.boxes.get_mut(view) {
                    sb.scroll_bottom();
                }
            }
        }
        Ok(())
    }

    async fn next(&mut self) -> Result<(), Error> {
        match &self.focused {
            FocusedView::List => {
                if !self.workshops.is_empty() {
                    let selected_index = self.titles_state.selected().unwrap_or(0);
                    let next_index = (selected_index + 1).min(self.workshops.len() - 1);
                    self.titles_state.select(Some(next_index));
                    self.cache_selected().await?;
                }
            }
            view => {
                if let Some(sb) = self.boxes.get_mut(view) {
                    sb.scroll_down();
                }
            }
        }
        Ok(())
    }

    async fn prev(&mut self) -> Result<(), Error> {
        match &self.focused {
            FocusedView::List => {
                if !self.workshops.is_empty() {
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
            view => {
                if let Some(sb) = self.boxes.get_mut(view) {
                    sb.scroll_up();
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
        let selected_index = self.titles_state.selected().unwrap_or(0);
        self.get_workshop_keys().get(selected_index).cloned()
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
        let fg = match self.focused {
            FocusedView::List => Color::White,
            _ => Color::DarkGray,
        };

        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled("/ Select a Workshop /", Style::default().fg(fg)),
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

    /// render the workshop info
    fn render_workshop_info(&mut self, area: Rect, buf: &mut Buffer) {
        let areas: [Rect; 3] = Layout::vertical([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .flex(Flex::End)
        .areas(area);

        self.render_workshop_box(areas[0], buf, FocusedView::Metadata, TOP_BOX_BORDER);
        self.render_workshop_box(areas[1], buf, FocusedView::Description, BOTTOM_BOX_BORDER);
        self.render_workshop_box(
            areas[2],
            buf,
            FocusedView::SetupInstructions,
            BOTTOM_BOX_BORDER,
        );
    }

    // render the workshop box
    fn render_workshop_box(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        view: FocusedView,
        border_set: Set,
    ) {
        if let Some(b) = self.boxes.get_mut(&view) {
            let fg = if self.focused == view {
                Color::White
            } else {
                Color::DarkGray
            };

            let title = Line::from(vec![
                Span::styled("─", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("/ {} /", view), Style::default().fg(fg)),
            ]);
            let block = Block::default()
                .title(title)
                .title_style(Style::default().fg(fg))
                .padding(Padding::top(1))
                .style(Style::default().fg(Color::DarkGray))
                .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
                .border_set(border_set);

            b.block(block);
            b.style(Style::default().fg(Color::White));

            // render the scroll text
            Widget::render(b, area, buf);
        }
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
                        info!("Show license: {}", license);
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
                            FocusedView::List => FocusedView::SetupInstructions,
                            FocusedView::Metadata => FocusedView::List,
                            FocusedView::Description => FocusedView::Metadata,
                            FocusedView::SetupInstructions => FocusedView::Description,
                        };
                    } else {
                        // switch focus to the next view
                        self.focused = match self.focused {
                            FocusedView::List => FocusedView::Metadata,
                            FocusedView::Metadata => FocusedView::Description,
                            FocusedView::Description => FocusedView::SetupInstructions,
                            FocusedView::SetupInstructions => FocusedView::List,
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
