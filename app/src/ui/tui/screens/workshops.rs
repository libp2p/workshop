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

    // get the selected workshop key
    fn get_selected_workshop_key(&self) -> Option<String> {
        if self.workshops.is_empty() {
            return None;
        }

        let selected_index = self.titles_state.selected().unwrap_or(0);
        self.get_workshop_keys().get(selected_index).cloned()
    }

    /// get the currently selected workshop
    fn get_selected_workshop(&self) -> Option<&Workshop> {
        let workshop_key = self.get_selected_workshop_key()?;
        self.workshops.get(workshop_key.as_str())
    }

    /// get the sorted list of workshop keys
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
        if self.workshops.is_empty() {
            return;
        }

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
        let workshop = match self.get_selected_workshop() {
            Some(workshop) => workshop,
            None => return,
        };

        let mut details = String::new();
        details.push_str("Authors: \n");
        details.push_str(
            &workshop
                .authors
                .iter()
                .map(|a| format!(" - {a}"))
                .collect::<Vec<_>>()
                .join(", "),
        );
        details.push_str("\nCopyright: ");
        details.push_str(&workshop.copyright);
        details.push_str("\nLicense: ");
        details.push_str(&workshop.license);
        details.push_str("\nHomepage: ");
        details.push_str(&workshop.homepage);
        details.push_str("\nDifficulty: ");
        details.push_str(&workshop.difficulty);
        /*
        details.push_str("\n\n");
        details.push_str(&workshop.description);
        details.push_str("\n\n");
        details.push_str(&workshop.setup);
        */

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
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    if let Some(workshop_key) = self.get_selected_workshop_key() {
                        info!("Get license: {}", workshop_key);
                        to_engine
                            .send(Message::GetLicense { name: workshop_key })
                            .await?;
                        return Ok(Some(UiEvent::ShowLicense));
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
                        info!("Open homepage: {}", workshop.homepage);
                        return Ok(Some(UiEvent::Homepage(workshop.homepage.clone())));
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
                        info!("Workshop selected: {}", workshop_key);
                        to_engine
                            .send(Message::SetWorkshop {
                                name: workshop_key.clone(),
                            })
                            .await?;
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
        if let Message::SelectWorkshop {
            workshops,
            spoken_language,
            programming_language,
        } = msg
        {
            info!("Showing select workshop screen");
            self.set_workshops(&workshops, spoken_language, programming_language);
            return Ok(Some(UiEvent::SelectWorkshop));
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
