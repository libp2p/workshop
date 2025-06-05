use crate::{
    fs,
    languages::programming,
    ui::tui::{
        self,
        screens::{self, Screens},
        Evt, Screen,
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
    widgets::{
        block::Position, Block, Borders, Clear, List, ListState, Padding, StatefulWidget, Widget,
    },
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use tracing::{debug, info};

const TOP_DIALOG_BORDER: Set = Set {
    top_left: "┌",
    top_right: "┐",
    bottom_left: "│",
    bottom_right: "│",
    vertical_left: "│",
    vertical_right: "│",
    horizontal_top: "─",
    horizontal_bottom: " ",
};

const STATUS_BORDER: Set = Set {
    top_left: " ",
    top_right: " ",
    bottom_left: "└",
    bottom_right: "┘",
    vertical_left: " ",
    vertical_right: " ",
    horizontal_top: " ",
    horizontal_bottom: "─",
};

#[derive(Clone, Debug, Default)]
pub struct Programming<'a> {
    /// the programming language list
    programming_languages: Vec<programming::Code>,
    /// the currenttly selected programming language
    programming_language: Option<programming::Code>,
    /// allow "Any" choice
    allow_any: bool,
    /// the event to pass to the SetProgrammingLanguage event
    event: Option<Evt>,
    /// the vertical lines of the dialog,
    lines: u16,
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
    /// set initialize the screen
    async fn init(
        &mut self,
        programming_languages: &[programming::Code],
        programming_language: Option<programming::Code>,
        allow_any: bool,
        event: Option<Evt>,
    ) -> Result<(), Error> {
        self.programming_languages = programming_languages.to_vec();
        self.programming_language = programming_language;
        self.allow_any = allow_any;
        self.event = event;

        // calculate the vertical lines of the dialog
        self.lines = self.selection_lines(programming_languages) + 4;

        // reset the cached rects so they get recalculated
        self.area = Rect::default();
        self.centered = Rect::default();

        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "/ Select a Programming Language /",
                Style::default().fg(Color::White),
            ),
        ]);
        self.list = List::new(self.language_names())
            .block(
                Block::default()
                    .title(title)
                    .title_style(Style::default().fg(Color::White))
                    .padding(Padding::uniform(1))
                    .style(Style::default().fg(Color::DarkGray))
                    .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
                    .border_set(TOP_DIALOG_BORDER),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::White))
            .highlight_symbol("> ");
        self.list_state
            .select(self.selection_from_language(self.programming_language));

        Ok(())
    }

    fn selection_lines<T, S: AsRef<[T]>>(&self, s: S) -> u16 {
        // If "Any" is allowed, we add one more line for the "Any" option
        if self.allow_any {
            s.as_ref().len() as u16 + 1
        } else {
            s.as_ref().len() as u16
        }
    }

    fn lang_to_selection(&self, index: usize) -> usize {
        if self.allow_any {
            // If "Any" is allowed, the index is shifted by 1
            index + 1 // shift other indices by 1
        } else {
            index
        }
    }

    fn selection_to_lang(&self, index: usize) -> usize {
        if self.allow_any {
            // If "Any" is allowed, the index is shifted back by 1
            index.saturating_sub(1)
        } else {
            index
        }
    }

    fn language_names(&self) -> Vec<String> {
        let mut names = if self.allow_any {
            vec!["Any".to_string()]
        } else {
            vec![]
        };
        names.extend(
            self.programming_languages
                .iter()
                .map(|code| code.get_name().to_string()),
        );
        names
    }

    fn language_from_selection(&self, index: usize) -> Option<programming::Code> {
        if index == 0 && self.allow_any {
            // If "Any" is selected, return None
            None
        } else {
            // Otherwise, get the programming language from the list
            self.programming_languages
                .get(self.selection_to_lang(index))
                .cloned()
        }
    }

    fn selection_from_language(&self, lang: Option<programming::Code>) -> Option<usize> {
        match lang {
            Some(code) => match self.programming_languages.iter().position(|&c| c == code) {
                Some(index) => Some(self.lang_to_selection(index)),
                None => Some(0),
            },
            None => Some(0),
        }
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
                Constraint::Length(self.lines),
                Constraint::Fill(1),
            ])
            .areas(hc);
            self.area = area;
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
        let line = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "/ j,k scroll / ↵ select /",
                Style::default().fg(Color::White),
            ),
        ]);
        let block = Block::default()
            .title(line)
            .title_style(Style::default().fg(Color::White))
            .title_position(Position::Bottom)
            .title_alignment(Alignment::Left)
            .style(Style::default().fg(Color::DarkGray))
            .borders(Borders::LEFT | Borders::BOTTOM | Borders::RIGHT)
            .border_set(STATUS_BORDER)
            .padding(Padding::horizontal(1));

        Widget::render(block, area, buf);
    }

    /// handle UI events
    pub async fn handle_ui_event(
        &mut self,
        event: tui::Event,
        to_ui: Sender<screens::Event>,
        _status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        match event {
            tui::Event::ChangeProgrammingLanguage(programming, allow_any, next) => {
                info!("Changing programming language");
                self.init(
                    &fs::application::all_programming_languages()?,
                    programming,
                    allow_any,
                    next,
                )
                .await?;
                to_ui
                    .send((None, tui::Event::Show(screens::Screens::Programming)).into())
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
        _status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        if let event::Event::Key(key) = event {
            match key.code {
                KeyCode::PageUp => self.list_state.select_first(),
                KeyCode::PageDown => self.list_state.select_last(),
                KeyCode::Char('b') | KeyCode::Esc => {
                    to_ui
                        .send((Some(Screens::Workshops), tui::Event::LoadWorkshops).into())
                        .await?;
                }
                KeyCode::Char('j') | KeyCode::Down => self.list_state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Enter => {
                    // take the event leaving None in its place
                    let event = self.event.take();
                    if let Some(selected) = self.list_state.selected() {
                        let programming_language = self.language_from_selection(selected);
                        let set_programming_language = (
                            None,
                            tui::Event::SetProgrammingLanguage(
                                programming_language,
                                None, // None, because we don't know if it should be the default
                                event,
                            ),
                        );
                        to_ui.send(set_programming_language.into()).await?;
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

        let [list_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(self.centered);

        self.render_list(list_area, buf);
        self.render_status(status_area, buf);
        Ok(())
    }
}
