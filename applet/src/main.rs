// SPDX-License-Identifier: GPL-3.0-only

use log;

mod applet;
mod i18n;
mod config;
mod logic;
mod power_options;
mod cosmic_session;
mod session_manager;
mod applet_button;
mod applet_menu;
mod model;

fn main() -> cosmic::iced::Result {
    // Initialize logging
    simple_logger::init_with_env().unwrap();
    log::info!("Starting Cosmic Classic Menu Applet");

    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);
    
    cosmic::applet::run::<applet::Applet>(())
}

