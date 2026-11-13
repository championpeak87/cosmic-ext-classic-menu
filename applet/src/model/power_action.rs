use serde::{Deserialize, Serialize};

use crate::applet::Message;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PowerAction {
    Shutdown,
    Logout,
    Lock,
    Reboot,
    Suspend,
    /// Only offered where the system can hibernate, see
    /// [`crate::power_options::can_hibernate`].
    Hibernate,
}

impl PowerAction {
    /// Every action, in the order the power controls show them.
    pub const ALL: [PowerAction; 6] = [
        PowerAction::Logout,
        PowerAction::Suspend,
        PowerAction::Hibernate,
        PowerAction::Lock,
        PowerAction::Reboot,
        PowerAction::Shutdown,
    ];

    /// The actions shown by default. Hibernating is left to be turned on,
    /// as few systems can.
    pub const DEFAULT: [PowerAction; 5] = [
        PowerAction::Logout,
        PowerAction::Suspend,
        PowerAction::Lock,
        PowerAction::Reboot,
        PowerAction::Shutdown,
    ];

    /// Whether the action can be offered, `can_hibernate` telling whether
    /// the system can hibernate.
    pub fn is_available(self, can_hibernate: bool) -> bool {
        self != PowerAction::Hibernate || can_hibernate
    }

    /// The action's bundled symbolic icon.
    pub fn icon(self) -> &'static [u8] {
        match self {
            PowerAction::Logout => {
                include_bytes!("../../../res/icons/bundled/system-log-out-symbolic.svg")
            }
            PowerAction::Suspend => {
                include_bytes!("../../../res/icons/bundled/system-suspend-symbolic.svg")
            }
            PowerAction::Hibernate => {
                include_bytes!("../../../res/icons/bundled/system-hibernate-symbolic.svg")
            }
            PowerAction::Lock => {
                include_bytes!("../../../res/icons/bundled/system-lock-screen-symbolic.svg")
            }
            PowerAction::Reboot => {
                include_bytes!("../../../res/icons/bundled/system-reboot-symbolic.svg")
            }
            PowerAction::Shutdown => {
                include_bytes!("../../../res/icons/bundled/system-shutdown-symbolic.svg")
            }
        }
    }

    /// The action's name, as its tooltip shows it.
    pub fn name(self) -> String {
        match self {
            PowerAction::Logout => crate::fl!("power-logout"),
            PowerAction::Suspend => crate::fl!("power-suspend"),
            PowerAction::Hibernate => crate::fl!("power-hibernate"),
            PowerAction::Lock => crate::fl!("power-lock"),
            PowerAction::Reboot => crate::fl!("power-reboot"),
            PowerAction::Shutdown => crate::fl!("power-shutdown"),
        }
    }

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
            PowerAction::Hibernate => {
                cosmic::iced::Task::perform(crate::power_options::hibernate(), msg)
            }
        }
    }
}
