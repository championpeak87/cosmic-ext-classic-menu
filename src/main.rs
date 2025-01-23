mod config;
mod window;
mod logic;

use crate::window::Window;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<Window>(())?;

    Ok(())
}
