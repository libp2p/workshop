use crate::{
    ui::tui::{widgets::ScrollText, Event as UiEvent, Screen},
    Error,
};
use crossterm::event::{Event, KeyCode};
use engine::{Lesson, Message};
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
pub struct Lessons<'a> {
    /// the list of lessons
    lessons: HashMap<String, Lesson>,
    /// the cached list
    titles: List<'a>,
    /// the list state of lesson title
    titles_state: ListState,
    /// the scrollable info window - requires lifetime for Block
    st: ScrollText<'a>,
    /// currently focused view
    focused: FocusedView,
}

impl Lessons<'_> {
    /// set the lessons
    fn set_lessons(&mut self, lessons: &HashMap<String, Lesson>) {
        self.lessons = lessons.clone();

        if self.lessons.is_empty() {
            self.titles_state.select(None);
        } else {
            self.titles_state.select_first();
        };

        // update the cached names
        let lesson_names = self
            .get_lesson_keys()
            .iter()
            .map(|name| self.lessons.get(name).unwrap().title.clone())
            .collect::<Vec<_>>();

        // create the list of lesson titles
        self.titles = List::new(lesson_names)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::White))
            .highlight_symbol("> ");
    }

    // get the selected lesson key
    fn get_selected_lesson_key(&self) -> Option<String> {
        if self.lessons.is_empty() {
            return None;
        }

        let selected_index = self.titles_state.selected().unwrap_or(0);
        self.get_lesson_keys().get(selected_index).cloned()
    }

    /// get the currently selected lesson
    fn get_selected_lesson(&self) -> Option<&Lesson> {
        let lesson_key = self.get_selected_lesson_key()?;
        self.lessons.get(lesson_key.as_str())
    }

    /// get the sorted list of lesson keys
    fn get_lesson_keys(&self) -> Vec<String> {
        let mut lesson_keys = self.lessons.keys().cloned().collect::<Vec<_>>();
        lesson_keys.sort();
        lesson_keys
    }

    /// render the lesson list and info
    fn render_lessons(&mut self, area: Rect, buf: &mut Buffer) {
        let [lesson_titles_area, lesson_info_area] =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .areas(area);

        self.render_lesson_titles(lesson_titles_area, buf);
        self.render_lesson_info(lesson_info_area, buf);
    }

    /// render the list of lesson titles
    fn render_lesson_titles(&mut self, area: Rect, buf: &mut Buffer) {
        // figure out the titles list border fg color based on what is focused
        let fg = match self.focused {
            FocusedView::List => Color::White,
            FocusedView::Info => Color::DarkGray,
        };

        let titles = self.titles.clone().block(
            Block::default()
                .title(" lessons ")
                .padding(Padding::horizontal(1))
                .style(Style::default().fg(fg))
                .borders(Borders::ALL),
        );

        StatefulWidget::render(&titles, area, buf, &mut self.titles_state);
    }

    /// render the lesson info
    fn render_lesson_info(&mut self, area: Rect, buf: &mut Buffer) {
        let mut description = match self.get_selected_lesson() {
            Some(lesson) => lesson.description.clone(),
            None => "No lessons support the selected spoken and programming languages".to_string(),
        };

        let fg = match self.focused {
            FocusedView::List => Color::DarkGray,
            FocusedView::Info => Color::White,
        };

        let block = Block::default()
            .title(" Description ")
            .padding(Padding::horizontal(1))
            .style(Style::default().fg(fg))
            .borders(Borders::ALL);

        self.st.block(block);
        self.st
            .style(Style::default().fg(Color::White).bg(Color::Black));

        // render the scroll text
        StatefulWidget::render(&mut self.st, area, buf, &mut description);
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

        let keys = Paragraph::new(
            "↓/↑ or j/k: scroll  |  tab: switch focus  |  enter: select  |  q: quit",
        )
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

        let fps = Paragraph::new(format!("[ {} | {} ]", spoken, programming))
            .block(block)
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Right);

        Widget::render(fps, area, buf);
    }
}

#[async_trait::async_trait]
impl Screen for Lessons<'_> {
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
                KeyCode::Tab => {
                    info!("Switch focus");
                    self.focused = match self.focused {
                        FocusedView::List => FocusedView::Info,
                        FocusedView::Info => FocusedView::List,
                    };
                }
                KeyCode::Enter => {
                    if let Some(lesson_key) = self.get_selected_lesson_key() {
                        info!("Lessons selected: {}", lesson_key);
                        to_engine
                            .send(Message::SetLesson {
                                name: lesson_key.clone(),
                            })
                            .await?;
                        return Ok(Some(UiEvent::SetLesson(lesson_key)));
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
        if let Message::SelectLesson { lessons } = msg {
            info!("Showing select lesson screen");
            self.set_lessons(&lessons);
            return Ok(Some(UiEvent::SelectLesson));
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
        let [lessons_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(2)])
                .flex(Flex::End)
                .areas(area);

        self.render_lessons(lessons_area, buf);
        self.render_status(status_area, buf, last_frame_duration);

        Ok(())
    }
}
