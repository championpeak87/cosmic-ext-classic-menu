mod config;
mod window;
mod logic;
mod spawn_detached;
mod launcher;

use crate::window::Window;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<Window>(())?;

    Ok(())
}
