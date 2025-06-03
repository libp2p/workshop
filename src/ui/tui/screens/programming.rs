use crate::{
    languages::programming,
    ui::tui::{self, screens, Screen},
    Error,
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
use tokio::sync::mpsc::Sender;
use tracing::info;

#[derive(Clone, Debug, Default)]
pub struct Programming<'a> {
    /// the programming language list
    programming_languages: Vec<programming::Code>,
    /// the currenttly selected programming language
    programming_language: Option<programming::Code>,
    /// the cached rect from last render
    area: Rect,
    /// the cached calculated rect
    centered: Rect,
    /// the cached list
    list: List<'a>,
    /// programming language list state
    list_state: ListState,
}

impl Programming<'_> {
    /// set the programming language list
    async fn set_programming_languages(
        &mut self,
        programming_languages: &[programming::Code],
    ) -> Result<(), Error> {
        self.programming_languages = programming_languages.to_vec();

        let mut programming_language_names = vec!["Any".to_string()];
        programming_language_names.extend(
            programming_languages
                .iter()
                .map(|code| code.get_name().to_string()),
        );
        let select_index = match self.programming_language {
            Some(code) => match programming_languages.iter().position(|&c| c == code) {
                Some(index) => Some(index + 1),
                None => Some(0),
            },
            None => Some(0),
        };

        self.list_state.select(select_index);
        self.list = List::new(programming_language_names)
            .block(
                Block::default()
                    .title(" Programming Languages ")
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

    async fn set_programming_language(
        &mut self,
        programming_language: Option<programming::Code>,
    ) -> Result<(), Error> {
        self.programming_language = programming_language;
        let select_index = match programming_language {
            Some(code) => match self.programming_languages.iter().position(|&c| c == code) {
                Some(index) => Some(index + 1),
                None => Some(0),
            },
            None => Some(0),
        };
        self.list_state.select(select_index);
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

        let keys = Paragraph::new(
            " ↓/↑ or j/k: scroll  |  enter: select  |  PgUp: start  | PgDwn: end  |  b: back  |  q: quit",
        )
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
        _to_ui: Sender<screens::Event>,
    ) -> Result<(), Error> {
        match event {
            tui::Event::ProgrammingLanguage(programming_language) => {
                info!("programming language set: {:?}", programming_language);
                self.set_programming_language(programming_language).await?;
            }
            tui::Event::SetProgrammingLanguages(ref programming_languages) => {
                info!("Setting programming languages: {:?}", programming_languages);
                self.set_programming_languages(programming_languages)
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
    ) -> Result<(), Error> {
        if let event::Event::Key(key) = event {
            match key.code {
                KeyCode::PageUp => self.list_state.select_first(),
                KeyCode::PageDown => self.list_state.select_last(),
                KeyCode::Char('b') | KeyCode::Esc => {
                    info!("Back to previous screen");
                    to_ui.send(tui::Event::LoadWorkshops.into()).await?;
                }
                KeyCode::Char('j') | KeyCode::Down => self.list_state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Enter => {
                    if let Some(selected) = self.list_state.selected() {
                        let programming_language = if selected == 0 {
                            info!("programming language selected: Any");
                            None
                        } else {
                            match self.programming_languages.get(selected - 1) {
                                Some(code) => {
                                    info!("programming language selected: {:?}", code);
                                    Some(*code)
                                }
                                None => {
                                    info!("No programming language selected");
                                    None
                                }
                            }
                        };
                        to_ui
                            .send(tui::Event::ProgrammingLanguage(programming_language).into())
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
impl Screen for Programming<'_> {
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
