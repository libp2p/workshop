use crate::{
    languages::spoken,
    ui::tui::{
        self,
        events::Evt,
        screens,
        widgets::{ScrollLog, StatusBar, StatusMode},
        Screen,
    },
    Error, Status,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    symbols::border::Set,
    text::{Line, Span},
    widgets::{block::Position, Block, Borders, Clear, Padding, StatefulWidget, Widget},
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, OnceLock},
};
use tokio::sync::mpsc::Sender;

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

// maps the log line prefix to the associated emoji
static EMOJIS: OnceLock<HashMap<&'static str, String>> = OnceLock::new();

fn emoji() -> &'static HashMap<&'static str, String> {
    EMOJIS.get_or_init(|| {
        let mut map = HashMap::new();
        map.insert("* ", "⭐".to_string());
        map.insert("v ", "✅".to_string());
        map.insert("x ", "❌".to_string());
        map.insert("r ", "🚀".to_string());
        map.insert("y ", "🎉".to_string());
        map.insert("n ", "😢".to_string());
        map.insert("! ", "❗".to_string());
        map.insert("^ ", "⚠️ ".to_string());
        map.insert("i ", "ℹ️ ".to_string());
        map.insert("> ", "↪️ ".to_string());
        map.insert("< ", "↩️ ".to_string());
        map
    })
}

#[derive(Clone, Debug)]
pub struct Log<'a> {
    /// the log messages
    log: VecDeque<(Option<String>, String)>,
    /// max log length
    max_log: usize,
    /// scroll text widget
    st: ScrollLog<'a>,
    /// status bar widget
    sb: StatusBar<'a>,
    /// the cached rect from last render
    area: Rect,
    /// the cached calculated rect
    centered: Rect,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
    /// waiting on enter key press
    on_enter: Option<Evt>,
}

impl Log<'_> {
    /// Create a new log Screen
    pub fn new(max_log: usize) -> Self {
        let mut st = ScrollLog::default();
        st.scroll_newest();
        let mut sb = StatusBar::new();
        let block = Block::default()
            .padding(Padding::horizontal(1))
            .style(Style::default().fg(Color::DarkGray))
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_set(TOP_DIALOG_BORDER);
        sb.set_block(block);

        Self {
            log: VecDeque::default(),
            max_log,
            st,
            sb,
            area: Rect::default(),
            centered: Rect::default(),
            spoken_language: None,
            on_enter: None,
        }
    }

    fn recalculate_rect(&mut self, area: Rect) {
        if self.area != area {
            let [_, hc, _] = Layout::horizontal([
                Constraint::Percentage(10),
                Constraint::Min(1),
                Constraint::Percentage(10),
            ])
            .areas(area);
            [_, self.centered, _] = Layout::vertical([
                Constraint::Percentage(10),
                Constraint::Min(1),
                Constraint::Percentage(10),
            ])
            .areas(hc);
            self.area = area;
        }
    }

    fn add_message<S: AsRef<str>>(&mut self, msg: S) {
        if msg.as_ref().len() < 2 {
            // if the message is too short, we can't determine the type
            return;
        }

        // add the message to the log
        let msg = msg.as_ref().to_string();
        self.log
            .push_back((emoji().get(&msg[0..2]).cloned(), msg[2..].to_string()));

        // if the log is too long, remove the oldest message
        if self.log.len() > self.max_log {
            self.log.pop_front();
        }
    }

    // render the log messages
    fn render_log(&mut self, area: Rect, buf: &mut Buffer) {
        // clear
        Widget::render(Clear, area, buf);

        let [log_area, status_bar_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)]).areas(area);

        let title = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            Span::styled("/ Log /", Style::default().fg(Color::White)),
        ]);

        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(Color::White))
            .padding(Padding::horizontal(1))
            .style(Style::default().fg(Color::DarkGray))
            .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
            .border_set(TOP_DIALOG_BORDER);

        self.st.block(block);
        self.st.style(Style::default().fg(Color::White));

        // render the scroll text
        StatefulWidget::render(&mut self.st, log_area, buf, &mut self.log);

        // render the command status line
        Widget::render(&mut self.sb, status_bar_area, buf);
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        let keys = if self.on_enter.is_some() {
            Span::styled(
                "/ j,k scroll / ⤒ top / ⤓ bottom / ↵ continue / q quit /",
                Style::default().fg(Color::White),
            )
        } else {
            Span::styled(
                "/ j,k scroll / ⤒ top / ⤓ bottom / ` back / q quit /",
                Style::default().fg(Color::White),
            )
        };
        let line = Line::from(vec![
            Span::styled("─", Style::default().fg(Color::DarkGray)),
            keys,
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
        _to_ui: Sender<screens::Event>,
        _status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        match event {
            tui::Event::Log(msg) => self.add_message(msg),
            tui::Event::CommandStarted(mode, message) => {
                match mode {
                    StatusMode::Blank => {
                        // Do nothing - StatusBar stays in Blank mode
                    }
                    StatusMode::Messages => {
                        self.sb.set_messages(message);
                    }
                    StatusMode::Progress => {
                        self.sb.set_progress(message);
                    }
                }
            }
            tui::Event::CommandOutput(message, progress) => {
                // Add to log as before
                self.add_message(&message);

                // Update status bar based on current mode
                if let Some(progress_val) = progress {
                    self.sb.update_progress(Some(message), progress_val);
                } else {
                    self.sb.update_message(message);
                }
            }
            tui::Event::CommandCompleted(result, success, failure) => {
                self.sb.set_blank();
                if result.success {
                    self.add_message(format!("y {}", result.last_line));
                    self.add_message("< Press ↵ Enter to continue");
                    self.on_enter = success;
                } else {
                    self.add_message(format!("n {}", result.last_line));
                    self.add_message("< Press ↵ Enter to continue");
                    self.on_enter = failure;
                }
            }
            _ => {}
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
                KeyCode::PageUp => self.st.scroll_oldest(),
                KeyCode::PageDown => self.st.scroll_newest(),
                KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => self.st.scroll_newer(),
                KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => self.st.scroll_older(),
                KeyCode::Char('`') => to_ui.send((None, tui::Event::ToggleLog).into()).await?,
                KeyCode::Enter => {
                    if let Some(on_enter) = self.on_enter.take() {
                        to_ui.send(on_enter.into()).await?
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Screen for Log<'_> {
    async fn handle_event(
        &mut self,
        event: screens::Event,
        to_ui: Sender<screens::Event>,
        status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        match event {
            screens::Event::Input(input_event) => {
                let spoken = {
                    let status = status.lock().unwrap();
                    status.spoken_language()
                };
                if self.spoken_language != spoken {
                    self.spoken_language = spoken
                }
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

        self.render_log(list_area, buf);
        self.render_status(status_area, buf);
        Ok(())
    }
}
