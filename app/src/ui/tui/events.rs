use crate::ui::tui::Popups;

/// UI events
#[derive(Clone, Debug)]
pub enum Event {
    Noop,
    Quit,
    ShowPopup(Popups),
    ClosePopup,
    Homepage(String),
}
