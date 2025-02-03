mod config;
mod window;
mod logic;
mod power_options;
mod cosmic_session;
mod session_manager;

use crate::window::Window;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<Window>(())?;

    Ok(())
}
