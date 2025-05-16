use crate::{
    ui::tui::{widgets::ScrollText, Event as UiEvent, EventHandler},
    Error,
};
use crossterm::event::{Event, KeyCode};
use engine::{Message, Workshop};
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

#[derive(Clone, Debug)]
pub struct Workshops<'a> {
    /// channel to the engine
    to_engine: Sender<Message>,
    /// the list of workshops
    workshops: HashMap<String, Workshop>,
    /// the list state of workshop title
    titles_state: ListState,
    /// the scrollable info window
    st: ScrollText<'a>,
    /// currently focused view
    focused: FocusedView,
}

impl Workshops<'_> {
    /// Create a new log screen
    pub fn new(to_engine: Sender<Message>) -> Self {
        Self {
            to_engine,
            workshops: HashMap::new(),
            titles_state: ListState::default(),
            st: ScrollText::default(),
            focused: FocusedView::List,
        }
    }

    /// set the workshops
    pub fn set_workshops(&mut self, workshops: &HashMap<String, Workshop>) {
        self.workshops = workshops.clone();
        if self.workshops.is_empty() {
            self.titles_state.select(None);
        } else {
            self.titles_state.select_first();
        };
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

    /// get the workshop titles sorted by keys
    fn get_workshop_names(&self) -> Vec<String> {
        self.get_workshop_keys()
            .iter()
            .map(|name| self.workshops.get(name).unwrap().title.clone())
            .collect::<Vec<_>>()
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

        let fg = match self.focused {
            FocusedView::List => Color::White,
            FocusedView::Info => Color::DarkGray,
        };

        // build a Text from workshop titles
        let workshop_names = self.get_workshop_names();

        // create the list of workshop titles
        let list = List::new(workshop_names)
            .block(
                Block::default()
                    .title(" Workshops ")
                    .padding(Padding::horizontal(1))
                    .style(Style::default().fg(fg))
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::White))
            .highlight_symbol("> ");

        StatefulWidget::render(list, area, buf, &mut self.titles_state);
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
        details.push_str("\n\n");
        details.push_str(&workshop.description);
        details.push_str("\n\n");
        details.push_str(&workshop.setup);

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
    fn render_status(&mut self, area: Rect, buf: &mut Buffer, state: &mut Duration) {
        // render the status bar at the bottom
        let [keys_area, fps_area] =
            Layout::horizontal([Constraint::Min(1), Constraint::Length(12)]).areas(area);

        self.render_keys(keys_area, buf);
        self.render_fps(fps_area, buf, state);
    }

    // render the keyboard shortcuts
    fn render_keys(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(1));

        let keys = Paragraph::new(
                "↓/↑ or j/k: scroll  |  tab: switch focus  |  enter: select\nw: homepage  |  l: license  |  s: spoken lang  |  p: programming lang  |  q: quit"
            )
            .block(block)
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);

        Widget::render(keys, area, buf);
    }

    // render the frames per second
    fn render_fps(&mut self, area: Rect, buf: &mut Buffer, state: &mut Duration) {
        let fps = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .title_alignment(Alignment::Right)
            .padding(Padding::horizontal(1))
            .title_bottom(format!("FPS: {:.2} ", 1.0 / state.as_secs_f64()));

        Widget::render(fps, area, buf);
    }
}

#[async_trait::async_trait]
impl EventHandler for &mut Workshops<'_> {
    async fn handle_event(&mut self, evt: &Event) -> Result<Option<UiEvent>, Error> {
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
                    if let Some(workshop) = self.get_selected_workshop() {
                        info!("Show license");
                        return Ok(Some(UiEvent::ShowLicense(workshop.license_text.clone())));
                    }
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
                        self.to_engine
                            .send(Message::SetWorkshop { name: workshop_key })
                            .await?;
                    }
                }
                _ => {}
            }
        }

        Ok(None)
    }
}

impl StatefulWidget for &mut Workshops<'_> {
    type State = Duration;

    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer, state: &mut Self::State) {
        // this splits the screen into a top area and a one-line bottom area
        let [workshops_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(2)])
                .flex(Flex::End)
                .areas(area);

        self.render_workshops(workshops_area, buf);
        self.render_status(status_area, buf, state);
    }
}
