// SPDX-License-Identifier: {{ license }}

mod app;
mod i18n;
mod layout_editor;
mod shortcut;

fn main() -> cosmic::iced::Result {
    // Initialize logging
    simple_logger::init_with_env().unwrap();
    log::info!("Starting Classic Menu Settings");

    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);
    // Place names come from the applet's translations.
    cosmic_ext_classic_menu_applet::i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default()
        // Wide enough for the layout editor next to the other settings.
        .size(cosmic::iced::Size::new(1200.0, 860.0))
        // The largest preview of the menu fits this width.
        .size_limits(cosmic::iced::Limits::NONE.min_width(600.0).min_height(400.0));

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<app::AppModel>(settings, ())
}
