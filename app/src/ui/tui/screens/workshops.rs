use crate::{
    ui::tui::{widgets::ScrollText, Event as UiEvent, Screen},
    Error,
};
use crossterm::event::{Event, KeyCode};
use engine::{Message, Workshop};
use languages::{programming, spoken};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListState, Padding, Paragraph, StatefulWidget, Widget, Wrap},
};
use std::{collections::HashMap, time::Duration};
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
    workshops: HashMap<String, Workshop>,
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
    fn set_workshops(
        &mut self,
        workshops: &HashMap<String, Workshop>,
        spoken_language: Option<spoken::Code>,
        programming_language: Option<programming::Code>,
    ) {
        self.workshops = workshops.clone();
        self.spoken_language = spoken_language;
        self.programming_language = programming_language;

        if self.workshops.is_empty() {
            self.titles_state.select(None);
        } else {
            self.titles_state.select_first();
        };

        // update the cached names
        let workshop_names = self
            .get_workshop_keys()
            .iter()
            .map(|name| self.workshops.get(name).unwrap().title.clone())
            .collect::<Vec<_>>();

        // create the list of workshop titles
        self.titles = List::new(workshop_names)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::White))
            .highlight_symbol("> ");
    }

    // set the description for the currently selected workshop
    fn set_description(&mut self, name: &str, description: &str) {
        self.descriptions
            .insert(name.to_string(), description.to_string());
    }

    // get the currently selected workshop
    fn get_selected_description(&self) -> Option<&String> {
        let workshop_key = self.get_selected_workshop_key()?;
        self.descriptions.get(workshop_key.as_str())
    }

    // set the setup instructions for the currently selected workshop
    fn set_setup_instructions(&mut self, name: &str, setup_instructions: &str) {
        self.setup_instructions
            .insert(name.to_string(), setup_instructions.to_string());
    }

    // get the currently selected workshop
    fn get_selected_setup_instructions(&self) -> Option<&String> {
        let workshop_key = self.get_selected_workshop_key()?;
        self.setup_instructions.get(workshop_key.as_str())
    }

    // set the spoken languages for the currently selected workshop
    fn set_spoken_languages(&mut self, name: &str, spoken_languages: &[spoken::Code]) {
        self.spoken_languages
            .insert(name.to_string(), spoken_languages.to_vec());
    }

    // get the currently selected workshop
    fn get_selected_spoken_languages(&self) -> Option<&Vec<spoken::Code>> {
        let workshop_key = self.get_selected_workshop_key()?;
        self.spoken_languages.get(workshop_key.as_str())
    }

    // set the programming languages for the currently selected workshop
    fn set_programming_languages(
        &mut self,
        name: &str,
        programming_languages: &[programming::Code],
    ) {
        self.programming_languages
            .insert(name.to_string(), programming_languages.to_vec());
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
        let workshop_key = self.get_selected_workshop_key()?;
        self.workshops.get(workshop_key.as_str())
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
    fn render_status(&mut self, area: Rect, buf: &mut Buffer, last_frame_duration: Duration) {
        // render the status bar at the bottom
        let [keys_area, fps_area] =
            Layout::horizontal([Constraint::Min(1), Constraint::Length(27)]).areas(area);

        self.render_keys(keys_area, buf);
        self.render_fps(fps_area, buf, last_frame_duration);
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

    // render the frames per second
    fn render_fps(&mut self, area: Rect, buf: &mut Buffer, last_frame_duration: Duration) {
        let block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .title_alignment(Alignment::Right)
            .padding(Padding::horizontal(1))
            .title_bottom(format!(
                "FPS: {:.2} ",
                1.0 / last_frame_duration.as_secs_f64()
            ));

        let spoken = match self.spoken_language {
            Some(code) => code.get_name_in_english().to_string(),
            None => "All".to_string(),
        };

        let programming = match self.programming_language {
            Some(code) => code.get_name().to_string(),
            None => "All".to_string(),
        };

        let fps = Paragraph::new(format!("[ {} | {} ]", spoken, programming))
            .block(block)
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Right);

        Widget::render(fps, area, buf);
    }
}

#[async_trait::async_trait]
impl Screen for Workshops<'_> {
    async fn handle_event(
        &mut self,
        evt: Event,
        _to_engine: Sender<Message>,
    ) -> Result<Option<UiEvent>, Error> {
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::PageUp => match self.focused {
                    FocusedView::List => {
                        self.titles_state.select_first();
                        if let Some(workshop_key) = self.get_selected_workshop_key() {
                            return Ok(Some(UiEvent::SelectWorkshop(workshop_key)));
                        }
                    }
                    FocusedView::Info => self.st.scroll_top(),
                },
                KeyCode::PageDown => match self.focused {
                    FocusedView::List => {
                        self.titles_state.select_last();
                        if let Some(workshop_key) = self.get_selected_workshop_key() {
                            return Ok(Some(UiEvent::SelectWorkshop(workshop_key)));
                        }
                    }
                    FocusedView::Info => self.st.scroll_bottom(),
                },
                KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => match self.focused {
                    FocusedView::List => {
                        self.titles_state.select_next();
                        if let Some(workshop_key) = self.get_selected_workshop_key() {
                            return Ok(Some(UiEvent::SelectWorkshop(workshop_key)));
                        }
                    }
                    FocusedView::Info => self.st.scroll_down(),
                },
                KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => match self.focused {
                    FocusedView::List => {
                        self.titles_state.select_previous();
                        if let Some(workshop_key) = self.get_selected_workshop_key() {
                            return Ok(Some(UiEvent::SelectWorkshop(workshop_key)));
                        }
                    }
                    FocusedView::Info => self.st.scroll_up(),
                },
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    if let Some(workshop_key) = self.get_selected_workshop_key() {
                        return Ok(Some(UiEvent::GetLicense(workshop_key)));
                    }
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    return Ok(Some(UiEvent::ChangeProgrammingLanguage))
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    return Ok(Some(UiEvent::ChangeSpokenLanguage))
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    if let Some(workshop) = self.get_selected_workshop() {
                        info!("(ui) Open homepage: {}", workshop.homepage);
                        return Ok(Some(UiEvent::Homepage(workshop.homepage.clone())));
                    }
                }
                KeyCode::Tab => {
                    info!("(ui) Switch focus");
                    self.focused = match self.focused {
                        FocusedView::List => FocusedView::Info,
                        FocusedView::Info => FocusedView::List,
                    };
                }
                KeyCode::Enter => {
                    if let Some(workshop_key) = self.get_selected_workshop_key() {
                        return Ok(Some(UiEvent::SetWorkshop(workshop_key)));
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
        match msg {
            Message::SelectWorkshop {
                workshops,
                spoken_language,
                programming_language,
            } => {
                info!("(ui) showing select workshop screen");
                self.set_workshops(&workshops, spoken_language, programming_language);
                if let Some(workshop_key) = self.get_selected_workshop_key() {
                    return Ok(Some(UiEvent::SelectWorkshop(workshop_key)));
                }
            }
            Message::ShowWorkshopDescription { ref name, ref text } => {
                info!("(ui) showing selected workshop description");
                self.set_description(name, text);
            }
            Message::ShowWorkshopSetupInstructions { ref name, ref text } => {
                info!("(ui) showing selected workshop setup instructions");
                self.set_setup_instructions(name, text);
            }
            Message::ShowWorkshopSpokenLanguages {
                ref name,
                ref spoken_languages,
            } => {
                info!("(ui) showing selected workshop spoken languages");
                self.set_spoken_languages(name, spoken_languages);
            }
            Message::ShowWorkshopProgrammingLanguages {
                ref name,
                ref programming_languages,
            } => {
                info!("(ui) showing selected workshop programming languages");
                self.set_programming_languages(name, programming_languages);
            }
            _ => info!("(ui) ignoring message: {}", msg),
        }
        Ok(None)
    }

    fn render_screen(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        last_frame_duration: Duration,
    ) -> Result<(), Error> {
        // this splits the screen into a top area and a one-line bottom area
        let [workshops_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(2)])
                .flex(Flex::End)
                .areas(area);

        self.render_workshops(workshops_area, buf);
        self.render_status(status_area, buf, last_frame_duration);

        Ok(())
    }
}
