use crate::{
    ui::tui::{self, screens, Screen},
    Error, Status,
};
use crossterm::event::{self, KeyCode};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Offset, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Widget, Wrap},
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

#[derive(Clone, Debug)]
pub struct Welcome<'a> {
    /// the cached rect from last render
    area: Rect,
    /// the cached calculated rect
    centered: Rect,
    /// the cached paragraph
    text: Paragraph<'a>,
}

impl Default for Welcome<'_> {
    fn default() -> Self {
        Self {
            area: Rect::default(),
            centered: Rect::default(),
            text: Paragraph::new("")
                .block(
                    Block::default()
                        .title(" Workshop v1.0 ")
                        .padding(Padding::horizontal(1))
                        .style(Style::default().fg(Color::White))
                        .borders(Borders::ALL),
                )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
        }
    }
}

impl Welcome<'_> {
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
                Constraint::Max(10),
                Constraint::Fill(1),
            ])
            .areas(hc);
        }
    }

    // render the dialog
    fn render_dialog(&mut self, area: Rect, buf: &mut Buffer) {
        // clear popup area
        Widget::render(Clear, area, buf);

        // render the list of programming language names
        Widget::render(&self.text, area, buf);
    }

    // render the status bar at the bottom
    fn render_status(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(1));

        let keys = Paragraph::new(" enter: Ok ")
            .block(block)
            .style(Style::default().fg(Color::Black).bg(Color::White))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);

        Widget::render(keys, area, buf);
    }
}

#[async_trait::async_trait]
impl Screen for Welcome<'_> {
    /// handle an input event
    async fn handle_event(
        &mut self,
        event: screens::Event,
        to_ui: Sender<screens::Event>,
        _status: Arc<Mutex<Status>>,
    ) -> Result<(), Error> {
        if let screens::Event::Input(event::Event::Key(key)) = event {
            if key.code == KeyCode::Enter {
                to_ui
                    .send((Some(screens::Screens::Workshops), tui::Event::LoadWorkshops).into())
                    .await?;
            }
        }
        Ok(())
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

        let [dialog_area, status_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                .flex(Flex::End)
                .areas(working_area);

        self.render_dialog(dialog_area, buf);
        self.render_status(status_area, buf);
        Ok(())
    }
}
