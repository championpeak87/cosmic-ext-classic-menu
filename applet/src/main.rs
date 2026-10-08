// SPDX-License-Identifier: GPL-3.0-only

use cosmic_ext_classic_menu_applet::{applet, i18n};

fn main() -> cosmic::iced::Result {
    // Initialize logging
    simple_logger::init_with_env().unwrap();
    log::info!("Starting Classic Menu Applet");

    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);
    
    cosmic::applet::run::<applet::Applet>(())
}

