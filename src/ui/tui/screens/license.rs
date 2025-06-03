use crate::{
    languages::spoken,
    ui::tui::{self, screens, widgets::ScrollText, Screen},
    Error,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Offset, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Padding, Paragraph, StatefulWidget, Widget, Wrap},
};
use tokio::sync::mpsc::Sender;
use tracing::info;

#[derive(Clone, Debug, Default)]
pub struct License<'a> {
    /// license text
    text: String,
    /// the cached rect from last render
    area: Rect,
    /// the cached calculated rect
    centered: Rect,
    /// scroll text widget
    st: ScrollText<'a>,
    /// the currently selected spoken language
    spoken_language: Option<spoken::Code>,
}

impl License<'_> {
    /// set the license text
    pub async fn set_license(&mut self, text: String) -> Result<(), Error> {
        self.text = text;
        Ok(())
    }

    async fn set_spoken_language(
        &mut self,
        spoken_language: Option<spoken::Code>,
    ) -> Result<(), Error> {
        self.spoken_language = spoken_language;
        Ok(())
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
        }
    }

    // render the log messages
    fn render_license(&mut self, area: Rect, buf: &mut Buffer) {
        Widget::render(Clear, area, buf);

        // render the list of license lines
        let block = Block::new()
            .title(Line::from(" License "))
            .padding(Padding::horizontal(1))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        self.st.block(block);
        self.st
            .style(Style::default().fg(Color::White).bg(Color::Black));

        // render the scroll text
        StatefulWidget::render(&mut self.st, area, buf, &mut self.text);
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(1));

        let keys = Paragraph::new(
            " ↓/↑ or j/k: scroll  |  PgUp: start  | PgDwn: end  |  b: back  |  q: quit",
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
            tui::Event::SpokenLanguage(spoken_language) => {
                info!("Spoken language set: {:?}", spoken_language);
                self.set_spoken_language(spoken_language).await?;
            }
            tui::Event::SetLicense(text) => {
                info!("Setting license text");
                self.set_license(text).await?;
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
                KeyCode::PageUp => self.st.scroll_top(),
                KeyCode::PageDown => self.st.scroll_bottom(),
                KeyCode::Char('b') | KeyCode::Esc => {
                    info!("Back to previous screen");
                    to_ui.send(tui::Event::LoadWorkshops.into()).await?;
                }
                KeyCode::Char('j') | KeyCode::Down => self.st.scroll_down(),
                KeyCode::Char('k') | KeyCode::Up => self.st.scroll_up(),
                _ => {}
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Screen for License<'_> {
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

        let [license_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(working_area);

        self.render_license(license_area, buf);
        self.render_status(status_area, buf);
        Ok(())
    }
}
