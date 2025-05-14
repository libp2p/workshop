use crate::{
    ui::tui::{Event as UiEvent, Popups},
    Error,
};
use crossterm::event::{Event, KeyCode};
use engine::{Message, Workshop};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, List, ListItem, ListState, Padding, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};
use std::{collections::HashMap, time::Duration};
use tokio::sync::mpsc::Sender;
use tracing::info;

#[derive(Clone, Debug)]
enum FocusedView {
    List,
    Info,
}

#[derive(Clone, Debug)]
pub struct Workshops {
    /// channel to the engine
    to_engine: Sender<Message>,
    /// the list of workshops
    workshops: HashMap<String, Workshop>,
    /// the list state of workshop title
    titles_state: ListState,
    /// info scroll
    info_scroll: u16,
    /// currently focused view
    focused: FocusedView,
    /// last frame duration
    last_frame_duration: Duration,
}

impl Workshops {
    /// Create a new log screen
    pub fn new(to_engine: Sender<Message>) -> Self {
        Self {
            to_engine,
            workshops: HashMap::new(),
            titles_state: ListState::default(),
            info_scroll: 0,
            focused: FocusedView::List,
            last_frame_duration: Duration::new(0, 0),
        }
    }

    /// set the last frame duration
    pub fn set_last_frame_duration(&mut self, duration: Duration) {
        self.last_frame_duration = duration;
    }

    /// set the workshops
    pub fn set_workshops(&mut self, workshops: HashMap<String, Workshop>) {
        self.workshops = workshops;
        if self.workshops.is_empty() {
            self.titles_state.select(None);
        } else {
            self.titles_state.select_first();
        };
    }

    /// handle an input event
    pub async fn handle_event(&mut self, evt: Event) -> Result<UiEvent, Error> {
        match evt {
            Event::Key(key) => match key.code {
                KeyCode::Char('`') => Ok(UiEvent::ShowPopup(Popups::Log)),
                KeyCode::Char('q') | KeyCode::Char('Q') => Ok(UiEvent::Quit),
                KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => {
                    match self.focused {
                        FocusedView::List => {
                            self.titles_state.select_next();
                        }
                        FocusedView::Info => {
                            self.info_scroll = self.info_scroll.saturating_add(1);
                        }
                    }
                    Ok(UiEvent::Noop)
                }
                KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => {
                    match self.focused {
                        FocusedView::List => {
                            self.titles_state.select_previous();
                        }
                        FocusedView::Info => {
                            self.info_scroll = self.info_scroll.saturating_sub(1);
                        }
                    }
                    Ok(UiEvent::Noop)
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    if self.workshops.is_empty() {
                        return Ok(UiEvent::Noop);
                    }

                    let selected = self.titles_state.selected().unwrap_or(0);
                    let workshop_key = self
                        .get_workshop_keys()
                        .get(selected)
                        .cloned()
                        .unwrap_or_default();

                    if let Some(workshop) = self.workshops.get(workshop_key.as_str()) {
                        info!("Show license");
                        Ok(UiEvent::ShowPopup(Popups::License(
                            workshop.license_text.clone(),
                        )))
                    } else {
                        Ok(UiEvent::Noop)
                    }
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    if self.workshops.is_empty() {
                        return Ok(UiEvent::Noop);
                    }

                    let selected = self.titles_state.selected().unwrap_or(0);
                    let workshop_key = self
                        .get_workshop_keys()
                        .get(selected)
                        .cloned()
                        .unwrap_or_default();

                    if let Some(workshop) = self.workshops.get(workshop_key.as_str()) {
                        info!("Open homepage: {}", workshop.homepage);
                        Ok(UiEvent::Homepage(workshop.homepage.clone()))
                    } else {
                        Ok(UiEvent::Noop)
                    }
                }
                KeyCode::Tab => {
                    info!("Switch focus");
                    self.focused = match self.focused {
                        FocusedView::List => FocusedView::Info,
                        FocusedView::Info => FocusedView::List,
                    };
                    Ok(UiEvent::Noop)
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    info!("Change spoken language");
                    self.to_engine.send(Message::ChangeSpokenLanguage).await?;
                    Ok(UiEvent::Noop)
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    info!("Change programming language");
                    self.to_engine
                        .send(Message::ChangeProgrammingLanguage)
                        .await?;
                    Ok(UiEvent::Noop)
                }
                KeyCode::Enter => {
                    if self.workshops.is_empty() {
                        return Ok(UiEvent::Noop);
                    }

                    let selected = self.titles_state.selected().unwrap_or(0);
                    let workshop_key = self
                        .get_workshop_keys()
                        .get(selected)
                        .cloned()
                        .unwrap_or_default();

                    info!("Workshop selected: {}", workshop_key);
                    if self.workshops.contains_key(workshop_key.as_str()) {
                        self.to_engine
                            .send(Message::SetWorkshop { name: workshop_key })
                            .await?;
                    }
                    Ok(UiEvent::Noop)
                }
                _ => Ok(UiEvent::Noop),
            },
            _ => Ok(UiEvent::Noop),
        }
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
        let fg = match self.focused {
            FocusedView::List => Color::White,
            FocusedView::Info => Color::DarkGray,
        };

        // build a Text from workshop titles
        let workshop_names = self
            .get_workshop_keys()
            .iter()
            .filter(|name| self.workshops.contains_key(name.as_str()))
            .map(|name| ListItem::from(self.workshops.get(name).unwrap().title.clone()))
            .collect::<Vec<_>>();

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
            .highlight_symbol("> ");

        StatefulWidget::render(list, area, buf, &mut self.titles_state);
    }

    /// render the workshop info
    fn render_workshop_info(&mut self, area: Rect, buf: &mut Buffer) {
        let fg = match self.focused {
            FocusedView::List => Color::DarkGray,
            FocusedView::Info => Color::White,
        };

        // get the currently highlighted index
        let selected_index = match self.titles_state.selected() {
            Some(index) => index,
            None => return,
        };

        // get the currently selected workshop
        let workshop = match self
            .workshops
            .get(&self.get_workshop_keys()[selected_index])
        {
            Some(workshop) => workshop,
            None => return,
        };

        let mut details = vec![
            Line::from(vec![
                Span::styled("Authors: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(workshop.authors.join(", ")),
            ]),
            Line::from(vec![
                Span::styled("Copyright: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&workshop.copyright),
            ]),
            Line::from(vec![
                Span::styled("License: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&workshop.license),
            ]),
            Line::from(vec![
                Span::styled("Homepage: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&workshop.homepage),
            ]),
            Line::from(vec![
                Span::styled(
                    "Difficulty: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(&workshop.difficulty),
            ]),
        ];

        // add the description
        let mut description = vec![
            Line::from(Span::raw("")),
            Line::from(vec![Span::styled(
                "Description: ",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(Span::raw("")),
        ];
        description.append(
            &mut workshop
                .description
                .trim()
                .split('\n')
                .map(|line| Line::from(Span::raw(line)))
                .collect::<Vec<_>>(),
        );
        details.append(&mut description);

        // add the setup instructions
        let mut setup = vec![
            Line::from(Span::raw("")),
            Line::from(vec![Span::styled(
                "Setup: ",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(Span::raw("")),
        ];
        setup.append(
            &mut workshop
                .setup
                .trim()
                .split('\n')
                .map(|line| Line::from(Span::raw(line)))
                .collect::<Vec<_>>(),
        );
        details.append(&mut setup);

        let details_paragraph = Paragraph::new(Text::from(details.clone()))
            .block(
                Block::default()
                    .title(" Details ")
                    .padding(Padding::horizontal(1))
                    .style(Style::default().fg(fg))
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false });

        Widget::render(details_paragraph, area, buf);

        // create the scrollbar state
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(details.len())
            .viewport_content_length(area.height.into())
            .position(self.info_scroll.into());

        // create the scrollbar
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

        StatefulWidget::render(scrollbar, area, buf, &mut scrollbar_state);
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        // render the status bar at the bottom
        let [keys_area, fps_area] =
            Layout::horizontal([Constraint::Min(1), Constraint::Length(10)]).areas(area);

        self.render_keys(keys_area, buf);
        self.render_fps(fps_area, buf);
    }

    // render the keyboard shortcuts
    fn render_keys(&mut self, area: Rect, buf: &mut Buffer) {
        let keys = Paragraph::new(
                "↓/↑ or j/k: scroll  |  tab: switch focus  |  enter: select\nw: homepage  |  l: license  |  s: spoken lang  |  p: programming lang  |  q: quit"
            )
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);

        Widget::render(keys, area, buf);
    }

    // render the frames per second
    fn render_fps(&mut self, area: Rect, buf: &mut Buffer) {
        let fps = Paragraph::new(format!(
            "FPS: {:.2} ",
            1.0 / self.last_frame_duration.as_secs_f64()
        ))
        .style(Style::default().fg(Color::Black).bg(Color::White))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Right);

        Widget::render(fps, area, buf);
    }
}

impl Widget for &mut Workshops {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        // this splits the screen into a top area and a one-line bottom area
        let [workshops_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(2)])
                .flex(Flex::End)
                .areas(area);

        self.render_workshops(workshops_area, buf);
        self.render_status(status_area, buf);
    }
}
