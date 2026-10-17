// SPDX-License-Identifier: GPL-3.0-only

use cosmic_ext_classic_menu_applet::{applet, dbus, i18n};

const USAGE: &str = "\
Usage: cosmic-ext-classic-menu-applet [OPTION]

Without an option, runs the applet. COSMIC's panel does this.

Options:
  --toggle    Open the menu of the running applet, or close it if it is open
  -h, --help  Show this help";

fn main() -> cosmic::iced::Result {
    // Options run without logging, so only their own messages are printed.
    match std::env::args().nth(1).as_deref() {
        None => {}
        Some("--toggle") => {
            if let Err(err) = dbus::toggle_popup() {
                eprintln!("Failed to toggle the menu, is the applet running? {err}");
                std::process::exit(1);
            }
            return Ok(());
        }
        Some("-h" | "--help") => {
            println!("{USAGE}");
            return Ok(());
        }
        Some(arg) => {
            eprintln!("Unknown option: {arg}\n\n{USAGE}");
            std::process::exit(2);
        }
    }

    // Initialize logging
    simple_logger::init_with_env().unwrap();
    log::info!("Starting Classic Menu Applet");

    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);
    
    cosmic::applet::run::<applet::Applet>(())
}

