use crate::{
    fs,
    languages::spoken,
    ui::tui::{self, screens, Screen},
    Error, Status,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Offset, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Clear, List, ListState, Padding, Paragraph, StatefulWidget, Widget, Wrap,
    },
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use tracing::info;

#[derive(Clone, Debug, Default)]
pub struct Spoken<'a> {
    /// the spoken language list
    spoken_languages: Vec<spoken::Code>,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
    /// the cached rect from last render
    area: Rect,
    /// the cached calculated rect
    centered: Rect,
    /// the cached list
    list: List<'a>,
    /// spoken language list state
    list_state: ListState,
}

impl Spoken<'_> {
    /// set the spoken language list
    async fn set_spoken_languages(
        &mut self,
        spoken_languages: &[spoken::Code],
        spoken_language: Option<spoken::Code>,
    ) -> Result<(), Error> {
        self.spoken_languages = spoken_languages.to_vec();
        self.spoken_language = spoken_language;

        let mut spoken_language_names = vec!["Any".to_string()];
        spoken_language_names.extend(
            spoken_languages
                .iter()
                .map(|code| code.get_name_in_english().to_string()),
        );
        let selected_index = match self.spoken_language {
            Some(code) => match spoken_languages.iter().position(|&c| c == code) {
                Some(index) => Some(index + 1),
                None => Some(0),
            },
            None => Some(0),
        };

        self.list_state.select(selected_index);
        self.list = List::new(spoken_language_names)
            .block(
                Block::default()
                    .title(" Spoken Languages ")
                    .padding(Padding::horizontal(1))
                    .style(Style::default().fg(Color::White))
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

        Ok(())
    }

    fn recalculate_rect(&mut self, area: Rect) {
        if self.area != area {
            let [_, hc, _] = Layout::horizontal([
                Constraint::Fill(1),
                Constraint::Max(44),
                Constraint::Fill(1),
            ])
            .areas(area);
            [_, self.centered, _] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Percentage(33),
                Constraint::Fill(1),
            ])
            .areas(hc);
        }
    }

    // render the list
    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        // clear popup area
        Widget::render(Clear, area, buf);

        // render the list of programming language names
        StatefulWidget::render(&self.list, area, buf, &mut self.list_state);
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(1));

        let keys = Paragraph::new(" ↓/↑ or j/k: scroll  |  enter: select")
            .block(block)
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);

        Widget::render(keys, area, buf);
    }

    /// handle UI events
    pub async fn handle_ui_event(
        &mut self,
        event: tui::Event,
        to_ui: Sender<screens::Event>,
        status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        match event {
            tui::Event::ChangeSpokenLanguage => {
                info!("Changing spoken language");
                let spoken = {
                    let status = status.lock().unwrap();
                    status.spoken_language()
                };
                self.set_spoken_languages(&fs::application::all_spoken_languages()?, spoken)
                    .await?;
                to_ui
                    .send((None, tui::Event::Show(screens::Screens::Spoken)).into())
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
                KeyCode::PageUp => self.list_state.select_first(),
                KeyCode::PageDown => self.list_state.select_last(),
                KeyCode::Char('b') | KeyCode::Esc => {
                    info!("Back to previous screen");
                    to_ui
                        .send((Some(screens::Screens::Workshops), tui::Event::LoadWorkshops).into())
                        .await?;
                }
                KeyCode::Char('j') | KeyCode::Down => self.list_state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Enter => {
                    if let Some(selected) = self.list_state.selected() {
                        let spoken_language = if selected == 0 {
                            info!("Spoken language selected: Any");
                            None
                        } else {
                            match self.spoken_languages.get(selected - 1) {
                                Some(code) => {
                                    info!("Spoken language selected: {:?}", code);
                                    Some(*code)
                                }
                                None => {
                                    info!("No spoken language selected");
                                    None
                                }
                            }
                        };
                        to_ui
                            .send(
                                (None, tui::Event::SetSpokenLanguage(spoken_language, None)).into(),
                            )
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
impl Screen for Spoken<'_> {
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
        self.recalculate_rect(area);

        // clear area around the popup
        Widget::render(Clear, self.centered, buf);

        let centered_block = Block::default()
            .padding(Padding::uniform(2))
            .borders(Borders::NONE);
        let working_area = centered_block.inner(self.centered);

        // draw drop shadow
        let mut shadow_area = working_area;
        shadow_area = shadow_area.offset(Offset { x: 1, y: 1 });
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray).bg(Color::DarkGray));
        Widget::render(block, shadow_area, buf);

        let [list_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(working_area);

        self.render_list(list_area, buf);
        self.render_status(status_area, buf);
        Ok(())
    }
}
