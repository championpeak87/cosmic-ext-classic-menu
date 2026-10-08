// SPDX-License-Identifier: {{ license }}

mod app;
mod i18n;
mod layout_editor;

fn main() -> cosmic::iced::Result {
    // Initialize logging
    simple_logger::init_with_env().unwrap();
    log::info!("Starting Classic Menu Settings");

    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let size = cosmic::iced::Size::new(600.0, 720.0);
    let settings = cosmic::app::Settings::default()
        .size(size)
        .size_limits(cosmic::iced::Limits::new(size, size))
        .resizable(Some(0.0));

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<app::AppModel>(settings, ())
}
