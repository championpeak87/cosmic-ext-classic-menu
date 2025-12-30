#[derive(Clone, Debug, PartialEq)]
pub enum PopupType {
    MainMenu,
    ContextMenu,
}

impl Default for PopupType {
    fn default() -> Self {
        PopupType::MainMenu
    }
}