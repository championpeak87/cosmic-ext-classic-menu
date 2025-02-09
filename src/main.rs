mod config;
mod window;
mod logic;
mod power_options;
mod cosmic_session;
mod session_manager;
mod localize;

use crate::window::Window;


const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> cosmic::iced::Result {
    tracing_subscriber::fmt::init();
    let _ = tracing_log::LogTracer::init();

    tracing::info!("Starting cosmic-app-list with version {VERSION}");

    localize::localize();

    cosmic::applet::run::<Window>(())?;

    Ok(())
}
