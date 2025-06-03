use crate::{
    languages::{programming, spoken},
    models::{Workshop, WorkshopData},
    ui::tui::{self, screens, widgets::ScrollText, Screen},
    Error,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
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
pub struct Workshops<'a> {
    /// the list of workshops
    workshops: HashMap<String, WorkshopData>,
    /// the selected workshop metadata
    workshop: Option<Workshop>,
    /// the cached descriptions of the workshops
    descriptions: HashMap<String, String>,
    /// the cached setup instructions of the workshops
    setup_instructions: HashMap<String, String>,
    /// the cached spoken languages this workshop has been translated to
    spoken_languages: HashMap<String, Vec<spoken::Code>>,
    /// the cached programming languages this workshop has been ported to
    programming_languages: HashMap<String, Vec<programming::Code>>,
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
    async fn set_workshops(
        &mut self,
        workshops: &HashMap<String, WorkshopData>,
    ) -> Result<(), Error> {
        info!("Setting workshops: {}", workshops.len());
        self.workshops = workshops.clone();
        self.workshop = None;
        self.descriptions.clear();
        self.setup_instructions.clear();
        self.spoken_languages.clear();
        self.programming_languages.clear();

        // reset both languages, they will be set again automatically
        self.spoken_language = None;
        self.programming_language = None;

        if self.workshops.is_empty() {
            self.titles_state.select(None);
        } else {
            self.titles_state.select_first();
        };

        if let Some(workshop_key) = self.get_selected_workshop_key() {
            self.set_selected_workshop(workshop_key.clone()).await?;
        }

        for (workshop_key, workshop) in &self.workshops {
            // update the the cached spoken languages
            let spoken_languages = workshop.get_all_spoken_languages();
            self.spoken_languages
                .insert(workshop_key.clone(), spoken_languages);

            // update the cached programming languages
            let programming_languages = workshop.get_all_programming_languages();
            self.programming_languages
                .insert(workshop_key.clone(), programming_languages);
        }

        Ok(())
    }

    async fn set_spoken_language(
        &mut self,
        spoken_language: Option<spoken::Code>,
    ) -> Result<(), Error> {
        self.spoken_language = spoken_language;

        // update the cached titles
        let mut workshop_titles = Vec::with_capacity(self.workshops.len());
        for workshop in self.workshops.values() {
            let metadata = workshop.get_metadata(self.spoken_language).await?;
            workshop_titles.push(metadata.title.clone());
        }

        // create the list of workshop titles
        self.titles = List::new(workshop_titles)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::White))
            .highlight_symbol("> ");

        if let Some(workshop_key) = self.get_selected_workshop_key() {
            self.set_selected_workshop(workshop_key.clone()).await?;
        }

        Ok(())
    }

    async fn set_programming_language(
        &mut self,
        programming_language: Option<programming::Code>,
    ) -> Result<(), Error> {
        self.programming_language = programming_language;

        if let Some(workshop_key) = self.get_selected_workshop_key() {
            self.set_selected_workshop(workshop_key.clone()).await?;
        }

        Ok(())
    }

    async fn set_selected_workshop(&mut self, workshop_key: String) -> Result<(), Error> {
        if let Some(workshop_data) = self.workshops.get(&workshop_key) {
            // get the workshop metadata
            let workshop = workshop_data.get_metadata(self.spoken_language).await?;
            self.workshop = Some(workshop);

            // update the cached description
            let description = workshop_data.get_description(self.spoken_language).await?;
            self.descriptions.insert(workshop_key.clone(), description);

            // update the cached setup instructions
            let setup_instructions = workshop_data
                .get_setup_instructions(self.spoken_language, self.programming_language)
                .await?;
            self.setup_instructions
                .insert(workshop_key.clone(), setup_instructions);
        } else {
            self.workshop = None;
        }

        Ok(())
    }

    // get the currently selected workshop
    fn get_selected_description(&self) -> Option<&String> {
        let workshop_key = self.get_selected_workshop_key()?;
        self.descriptions.get(workshop_key.as_str())
    }

    // get the currently selected workshop
    fn get_selected_setup_instructions(&self) -> Option<&String> {
        let workshop_key = self.get_selected_workshop_key()?;
        self.setup_instructions.get(workshop_key.as_str())
    }

    // get the currently selected workshop
    fn get_selected_spoken_languages(&self) -> Option<&Vec<spoken::Code>> {
        let workshop_key = self.get_selected_workshop_key()?;
        self.spoken_languages.get(workshop_key.as_str())
    }

    // get the currently selected workshop
    fn get_selected_programming_languages(&self) -> Option<&Vec<programming::Code>> {
        let workshop_key = self.get_selected_workshop_key()?;
        self.programming_languages.get(workshop_key.as_str())
    }

    // get the selected workshop key
    fn get_selected_workshop_key(&self) -> Option<String> {
        if self.workshops.is_empty() {
            return None;
        }

        let selected_index = self.titles_state.selected().unwrap_or(0);
        self.get_workshop_keys().get(selected_index).cloned()
    }

    // get the currently selected workshop
    fn get_selected_workshop(&self) -> Option<&Workshop> {
        self.workshop.as_ref()
    }

    // get the sorted list of workshop keys
    fn get_workshop_keys(&self) -> Vec<String> {
        let mut workshop_keys = self.workshops.keys().cloned().collect::<Vec<_>>();
        workshop_keys.sort();
        workshop_keys
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
        match self.get_selected_workshop() {
            Some(workshop) => {
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
            }
            None => {
                details
                    .push_str("No workshops support the selected spoken and programming languages");
            }
        }
        if let Some(spoken_languages) = self.get_selected_spoken_languages() {
            details.push_str("\nSpoken Languages:\n");
            details.push_str(
                &spoken_languages
                    .iter()
                    .map(|c| format!(" - {}", c.get_name_in_native()))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        if let Some(programming_languages) = self.get_selected_programming_languages() {
            details.push_str("\nProgramming Languages:\n");
            details.push_str(
                &programming_languages
                    .iter()
                    .map(|c| format!(" - {}", c.get_name()))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        if let Some(description) = self.get_selected_description() {
            details.push_str("\n\n");
            details.push_str(description);
        }
        if let Some(setup_instructions) = self.get_selected_setup_instructions() {
            details.push_str("\n\n");
            details.push_str(setup_instructions);
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
        _to_ui: Sender<screens::Event>,
    ) -> Result<(), Error> {
        match event {
            tui::Event::SpokenLanguage(spoken_language) => {
                info!("Workshops Spoken language set: {:?}", spoken_language);
                self.set_spoken_language(spoken_language).await?;
            }
            tui::Event::ProgrammingLanguage(programming_language) => {
                info!(
                    "Workshops Programming language set: {:?}",
                    programming_language
                );
                self.set_programming_language(programming_language).await?;
            }
            // TODO: have this also pass the selected workshop for clean resuming
            tui::Event::SetWorkshops(workshops) => {
                info!("Setting workshops");
                self.set_workshops(&workshops).await?;
                if let Some(workshop_key) = self.get_selected_workshop_key() {
                    self.set_selected_workshop(workshop_key).await?;
                }
            }
            tui::Event::SelectWorkshop(workshop_key) => {
                info!("Selected workshop: {}", workshop_key);
                self.set_selected_workshop(workshop_key).await?;
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
                        if let Some(workshop_key) = self.get_selected_workshop_key() {
                            self.set_selected_workshop(workshop_key).await?;
                        }
                    }
                    FocusedView::Info => self.st.scroll_top(),
                },
                KeyCode::PageDown => match self.focused {
                    FocusedView::List => {
                        self.titles_state.select_last();
                        if let Some(workshop_key) = self.get_selected_workshop_key() {
                            self.set_selected_workshop(workshop_key).await?;
                        }
                    }
                    FocusedView::Info => self.st.scroll_bottom(),
                },
                KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => match self.focused {
                    FocusedView::List => {
                        info!("select next");
                        self.titles_state.select_next();
                        if let Some(workshop_key) = self.get_selected_workshop_key() {
                            self.set_selected_workshop(workshop_key).await?;
                        }
                    }
                    FocusedView::Info => self.st.scroll_down(),
                },
                KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => match self.focused {
                    FocusedView::List => {
                        info!("select previous");
                        self.titles_state.select_previous();
                        info!("selected previous");
                        if let Some(workshop_key) = self.get_selected_workshop_key() {
                            info!("Setting selected workshop: {}", workshop_key);
                            self.set_selected_workshop(workshop_key).await?;
                        }
                    }
                    FocusedView::Info => self.st.scroll_up(),
                },
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    if let Some(workshop_key) = self.get_selected_workshop_key() {
                        if let Some(workshop_data) = self.workshops.get(&workshop_key) {
                            info!("Show license for workshop: {}", workshop_key);
                            let license = workshop_data.get_license().await?;
                            to_ui.send(tui::Event::ShowLicense(license).into()).await?;
                        } else {
                            info!("No workshop data found for key: {}", workshop_key);
                        }
                    }
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    to_ui
                        .send(tui::Event::ChangeProgrammingLanguage.into())
                        .await?;
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    to_ui.send(tui::Event::ChangeSpokenLanguage.into()).await?;
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    if let Some(workshop) = self.get_selected_workshop() {
                        info!("Open homepage: {}", workshop.homepage);
                        to_ui
                            .send(tui::Event::Homepage(workshop.homepage.clone()).into())
                            .await?;
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
                            .send(tui::Event::LoadLessons(workshop_key).into())
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
    ) -> Result<(), Error> {
        match event {
            screens::Event::Input(input_event) => self.handle_input_event(input_event, to_ui).await,
            screens::Event::Ui(ui_event) => self.handle_ui_event(ui_event, to_ui).await,
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
