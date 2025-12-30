use crate::applet::Message;

#[derive(Clone, Debug, PartialEq)]
pub enum PowerAction {
    Shutdown,
    Logout,
    Lock,
    Reboot,
    Suspend,
}

impl PowerAction {
    pub fn perform(self) -> cosmic::iced::Task<cosmic::Action<Message>> {
        let msg = |m| cosmic::Action::App(Message::Zbus(m));
        match self {
            PowerAction::Lock => cosmic::iced::Task::perform(crate::power_options::lock(), msg),
            PowerAction::Logout => {
                cosmic::iced::Task::perform(crate::power_options::log_out(), msg)
            }
            PowerAction::Reboot => {
                cosmic::iced::Task::perform(crate::power_options::restart(), msg)
            }
            PowerAction::Shutdown => {
                cosmic::iced::Task::perform(crate::power_options::shutdown(), msg)
            }
            PowerAction::Suspend => {
                cosmic::iced::Task::perform(crate::power_options::suspend(), msg)
            }
        }
    }
}
