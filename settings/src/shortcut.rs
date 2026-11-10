// SPDX-License-Identifier: GPL-3.0-only

//! The keyboard shortcut opening the menu, kept with COSMIC's custom
//! shortcuts.

use cosmic::cosmic_config::{self, ConfigGet, ConfigSet};
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::{Key, Modifiers};
use cosmic_settings_config::shortcuts::{self, Action, Binding, Shortcuts};

/// Key of the user's shortcuts in COSMIC's shortcuts config.
const CUSTOM_KEY: &str = "custom";
const NATIVE_COMMAND: &str = "cosmic-ext-classic-menu-applet --toggle";
const FLATPAK_COMMAND: &str = "flatpak run com.championpeak87.cosmic-ext-classic-menu --toggle";

/// Why a shortcut couldn't be set.
#[derive(Debug)]
pub enum Error {
    /// Another shortcut uses these keys already.
    Taken,
    Config(cosmic_config::Error),
}

impl From<cosmic_config::Error> for Error {
    fn from(error: cosmic_config::Error) -> Self {
        Error::Config(error)
    }
}

/// The command opening the menu of this installation.
fn command() -> &'static str {
    if std::env::var_os("FLATPAK_ID").is_some() {
        FLATPAK_COMMAND
    } else {
        NATIVE_COMMAND
    }
}

fn opens_menu(action: &Action) -> bool {
    matches!(action, Action::Spawn(command) if command == NATIVE_COMMAND || command == FLATPAK_COMMAND)
}

/// The user's shortcuts, which are empty until the first one is added.
fn custom_shortcuts(context: &cosmic_config::Config) -> Result<Shortcuts, cosmic_config::Error> {
    match context.get::<Shortcuts>(CUSTOM_KEY) {
        Err(error) if error.is_err() => Err(error),
        Err(_) => Ok(Shortcuts::default()),
        Ok(custom) => Ok(custom),
    }
}

/// The shortcut currently opening the menu.
pub fn current() -> Option<Binding> {
    let context = shortcuts::context().ok()?;
    custom_shortcuts(&context)
        .ok()?
        .0
        .into_iter()
        .find_map(|(binding, action)| opens_menu(&action).then_some(binding))
}

/// Makes `binding` open the menu, or no shortcut at all without one.
///
/// Super alone may replace the system's launcher, which returns once the
/// shortcut is removed, but any other shortcut in use is left alone.
pub fn set(binding: Option<Binding>) -> Result<(), Error> {
    let context = shortcuts::context()?;
    if let Some(binding) = binding.as_ref()
        && is_taken(&shortcuts::shortcuts(&context), binding)
    {
        return Err(Error::Taken);
    }

    let custom = with_menu_shortcut(custom_shortcuts(&context)?, binding, command());
    context.set(CUSTOM_KEY, custom)?;
    Ok(())
}

fn is_taken(all: &Shortcuts, binding: &Binding) -> bool {
    !binding.is_super() && all.0.get(binding).is_some_and(|action| !opens_menu(action))
}

/// `custom` with `binding` as the only shortcut opening the menu.
fn with_menu_shortcut(mut custom: Shortcuts, binding: Option<Binding>, command: &str) -> Shortcuts {
    custom.0.retain(|_, action| !opens_menu(action));
    if let Some(binding) = binding {
        custom.0.insert(binding, Action::Spawn(command.to_owned()));
    }
    custom
}

/// The binding of Super alone.
pub fn super_key() -> Binding {
    Binding {
        modifiers: shortcuts::Modifiers::new().logo(),
        ..Binding::default()
    }
}

/// The binding of a key press, if it can be a shortcut. Modifiers alone
/// and plain typing keys can't.
pub fn binding_for(key: &Key, modifiers: Modifiers) -> Option<Binding> {
    let mut parts = Vec::new();
    if modifiers.logo() {
        parts.push("Super".to_owned());
    }
    if modifiers.control() {
        parts.push("Ctrl".to_owned());
    }
    if modifiers.alt() {
        parts.push("Alt".to_owned());
    }
    if modifiers.shift() {
        parts.push("Shift".to_owned());
    }
    parts.push(key_name(key)?);

    Binding::from_str_partial(&parts.join("+"))
        .ok()
        .filter(Binding::is_set)
}

/// Name of a key, as `Binding` understands it.
fn key_name(key: &Key) -> Option<String> {
    match key {
        // The separator of the keys in a binding.
        Key::Character(character) if character.as_str() == "+" => Some("plus".to_owned()),
        Key::Character(character) if character.as_str() == " " => Some("space".to_owned()),
        Key::Character(character) => Some(character.to_lowercase()),
        Key::Named(named) => named_key(*named),
        Key::Unidentified => None,
    }
}

fn named_key(named: Named) -> Option<String> {
    let name = match named {
        Named::Enter => "Return",
        Named::Tab => "Tab",
        Named::Backspace => "BackSpace",
        Named::Delete => "Delete",
        Named::Insert => "Insert",
        Named::Home => "Home",
        Named::End => "End",
        Named::PageUp => "Prior",
        Named::PageDown => "Next",
        Named::ArrowLeft => "Left",
        Named::ArrowRight => "Right",
        Named::ArrowUp => "Up",
        Named::ArrowDown => "Down",
        Named::Escape => "Escape",
        Named::Pause => "Pause",
        Named::PrintScreen => "Print",
        // The function keys are named alike.
        named => {
            let name = format!("{named:?}");
            let is_function_key = name
                .strip_prefix('F')
                .is_some_and(|number| number.parse::<u8>().is_ok());
            return is_function_key.then_some(name);
        }
    };
    Some(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn binding(keys: &str) -> Binding {
        Binding::from_str(keys).unwrap()
    }

    fn spawn(command: &str) -> Action {
        Action::Spawn(command.to_owned())
    }

    fn character(key: &str) -> Key {
        Key::Character(key.into())
    }

    #[test]
    fn replaces_the_previous_menu_shortcut_only() {
        let other = spawn("firefox");
        let custom = Shortcuts(
            [
                (binding("Super+m"), spawn(NATIVE_COMMAND)),
                (binding("Super+b"), other.clone()),
            ]
            .into(),
        );

        let custom = with_menu_shortcut(custom, Some(binding("Super+n")), NATIVE_COMMAND);

        assert_eq!(custom.0.len(), 2);
        assert_eq!(custom.0.get(&binding("Super+n")), Some(&spawn(NATIVE_COMMAND)));
        assert_eq!(custom.0.get(&binding("Super+b")), Some(&other));
    }

    #[test]
    fn removes_the_menu_shortcut() {
        let custom = Shortcuts([(binding("Super+m"), spawn(FLATPAK_COMMAND))].into());
        assert!(with_menu_shortcut(custom, None, FLATPAK_COMMAND).0.is_empty());
    }

    #[test]
    fn super_alone_may_replace_the_launcher() {
        let all = Shortcuts([(super_key(), Action::Disable)].into());
        assert!(!is_taken(&all, &super_key()));
    }

    #[test]
    fn keys_of_other_shortcuts_are_taken() {
        let all = Shortcuts(
            [
                (binding("Super+b"), spawn("firefox")),
                (binding("Super+m"), spawn(NATIVE_COMMAND)),
            ]
            .into(),
        );
        assert!(is_taken(&all, &binding("Super+b")));
        assert!(!is_taken(&all, &binding("Super+m")));
        assert!(!is_taken(&all, &binding("Super+n")));
    }

    #[test]
    fn combinations_become_bindings() {
        let modifiers = Modifiers::LOGO | Modifiers::SHIFT;
        assert_eq!(
            binding_for(&character("M"), modifiers),
            Some(binding("Super+Shift+m"))
        );
        assert_eq!(
            binding_for(&character(" "), Modifiers::CTRL),
            Some(binding("Ctrl+space"))
        );
        assert_eq!(
            binding_for(&character("+"), Modifiers::LOGO),
            Some(binding("Super+plus"))
        );
    }

    #[test]
    fn function_keys_work_alone() {
        assert_eq!(
            binding_for(&Key::Named(Named::F9), Modifiers::empty()),
            Some(binding("F9"))
        );
    }

    #[test]
    fn typing_keys_and_modifiers_alone_are_no_shortcuts() {
        assert_eq!(binding_for(&character("a"), Modifiers::empty()), None);
        assert_eq!(binding_for(&Key::Named(Named::Shift), Modifiers::SHIFT), None);
        assert_eq!(binding_for(&Key::Named(Named::Super), Modifiers::LOGO), None);
    }

    #[test]
    fn named_keys_are_valid_binding_names() {
        for named in [
            Named::Enter,
            Named::Backspace,
            Named::PageUp,
            Named::PageDown,
            Named::ArrowLeft,
            Named::PrintScreen,
            Named::F12,
        ] {
            assert!(
                binding_for(&Key::Named(named), Modifiers::CTRL).is_some(),
                "{named:?}"
            );
        }
    }
}
