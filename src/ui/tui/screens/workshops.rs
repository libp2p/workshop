use crate::{
    fs,
    languages::{programming, spoken},
    models::{Workshop, WorkshopData},
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
use tracing::{error, info};

#[derive(Clone, Debug, Default)]
enum FocusedView {
    #[default]
    List,
    Info,
}

#[derive(Clone, Debug)]
struct Cached {
    workshop: Workshop,
    languages: HashMap<spoken::Code, Vec<programming::Code>>,
    description: String,
    setup_instructions: String,
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
    /// the scrollable info window - requires lifetime for Block
    st: ScrollText<'a>,
    /// currently focused view
    focused: FocusedView,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
    /// the currently selected programming language
    programming_language: Option<programming::Code>,
}

impl Workshops<'_> {
    /// set the workshops
    async fn init(
        &mut self,
        workshops: &HashMap<String, WorkshopData>,
        spoken_language: Option<spoken::Code>,
        programming_language: Option<programming::Code>,
    ) -> Result<(), Error> {
        info!("Initializing workshops");
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
        info!("Caching workshop titles");
        self.titles_map.clear();
        for (key, wd) in self.workshops.iter() {
            let workshop = wd.get_metadata(self.spoken_language).await?;
            self.titles_map.insert(workshop.title.clone(), key.clone());
        }
        Ok(self.titles_map.keys().cloned().collect())
    }

    // cached selected workshop data
    async fn cache_selected(&mut self) -> Result<(), Error> {
        info!("Caching selected workshop data");
        self.selected = None;
        if let Some(workshop_key) = self.get_selected_workshop_key() {
            if let Some(workshop_data) = self.workshops.get(&workshop_key) {
                let workshop = workshop_data.get_metadata(self.spoken_language).await?;
                let languages = workshop_data.get_languages().clone();
                let description = workshop_data
                    .get_description(self.spoken_language)
                    .await
                    .unwrap_or_default();
                let setup_instructions = workshop_data
                    .get_setup_instructions(self.spoken_language, self.programming_language)
                    .await
                    .unwrap_or_default();
                let license = workshop_data.get_license().await?;
                self.selected = Some(Cached {
                    workshop,
                    languages,
                    description,
                    setup_instructions,
                    license,
                });
            }
        }
        Ok(())
    }

    // select first workshop
    async fn select_first(&mut self) -> Result<(), Error> {
        if !self.workshops.is_empty() {
            self.titles_state.select(Some(0));
            self.cache_selected().await?;
        }
        Ok(())
    }

    // select last workshop
    async fn select_last(&mut self) -> Result<(), Error> {
        if !self.workshops.is_empty() {
            let last_index = self.workshops.len() - 1;
            self.titles_state.select(Some(last_index));
            self.cache_selected().await?;
        }
        Ok(())
    }

    // select next workshop
    async fn select_next(&mut self) -> Result<(), Error> {
        if !self.workshops.is_empty() {
            let selected_index = self.titles_state.selected().unwrap_or(0);
            let next_index = (selected_index + 1).min(self.workshops.len() - 1);
            self.titles_state.select(Some(next_index));
            self.cache_selected().await?;
        }
        Ok(())
    }

    // select previous workshop
    async fn select_prev(&mut self) -> Result<(), Error> {
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
            FocusedView::Info => Color::DarkGray,
        };

        let titles = self.titles.clone().block(
            Block::default()
                .title(" Workshops ")
                .padding(Padding::horizontal(1))
                .style(Style::default().fg(fg))
                .borders(Borders::ALL),
        );

        StatefulWidget::render(&titles, area, buf, &mut self.titles_state);
    }

    /// render the workshop info
    fn render_workshop_info(&mut self, area: Rect, buf: &mut Buffer) {
        let mut details = String::new();
        match &self.selected {
            Some(Cached {
                workshop,
                languages,
                description,
                setup_instructions,
                ..
            }) => {
                details.push_str("Authors: \n");
                details.push_str(
                    &workshop
                        .authors
                        .iter()
                        .map(|a| format!(" - {a}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                details.push_str("\nCopyright: ");
                details.push_str(&workshop.copyright);
                details.push_str("\nLicense: ");
                details.push_str(&workshop.license);
                details.push_str("\nHomepage: ");
                details.push_str(&workshop.homepage);
                details.push_str("\nDifficulty: ");
                details.push_str(&workshop.difficulty);
                details.push_str("\nLanguages:\n");
                details.push_str(
                    &languages
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
                details.push_str("\n\n");
                details.push_str(description);
                details.push_str("\n\n");
                details.push_str(setup_instructions);
            }
            None => {
                details
                    .push_str("No workshops support the selected spoken and programming languages");
            }
        }

        let fg = match self.focused {
            FocusedView::List => Color::DarkGray,
            FocusedView::Info => Color::White,
        };

        let block = Block::default()
            .title(" Details ")
            .padding(Padding::horizontal(1))
            .style(Style::default().fg(fg))
            .borders(Borders::ALL);

        self.st.block(block);
        self.st
            .style(Style::default().fg(Color::White).bg(Color::Black));

        // render the scroll text
        StatefulWidget::render(&mut self.st, area, buf, &mut details);
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
        let block = Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(1));

        let lines = [
            "↓/↑ or j/k: scroll  |  tab: switch focus  |  enter: select",
            "w: homepage  |  l: license  |  s: spoken lang  |  p: programming lang  |  q: quit",
        ];

        let keys = Paragraph::new(lines.join("\n"))
            .block(block)
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);

        Widget::render(keys, area, buf);
    }

    // render the selected languages
    fn render_lang(&mut self, area: Rect, buf: &mut Buffer) {
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
            tui::Event::LoadWorkshops => {
                info!("Loading workshops");
                let (spoken, programming) = {
                    let status = status.lock().unwrap();
                    (status.spoken_language(), status.programming_language())
                };
                let workshops = fs::application::all_workshops_filtered(spoken, programming)?;
                self.init(&workshops, spoken, programming).await?;
                to_ui
                    .send((None, tui::Event::Show(screens::Screens::Workshops)).into())
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
                        info!("No selected workshop");
                    }
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    to_ui
                        .send(
                            (
                                Some(screens::Screens::Programming),
                                tui::Event::ChangeProgrammingLanguage,
                            )
                                .into(),
                        )
                        .await?;
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    to_ui
                        .send(
                            (
                                Some(screens::Screens::Spoken),
                                tui::Event::ChangeSpokenLanguage,
                            )
                                .into(),
                        )
                        .await?;
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
                    info!("Switch focus");
                    self.focused = match self.focused {
                        FocusedView::List => FocusedView::Info,
                        FocusedView::Info => FocusedView::List,
                    };
                }
                KeyCode::Enter => {
                    if let Some(workshop_key) = self.get_selected_workshop_key() {
                        info!("Selected workshop: {}", workshop_key);
                        to_ui
                            .send((None, tui::Event::SetWorkshop(workshop_key)).into())
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
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(2)])
                .flex(Flex::End)
                .areas(area);

        self.render_workshops(workshops_area, buf);
        self.render_status(status_area, buf);

        Ok(())
    }
}
