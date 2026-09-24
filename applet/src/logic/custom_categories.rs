//! Support for custom application categories defined by third party applications
//! (Wine, Citrix Workspace, Cisco Secure Client, ...) through merged XDG menu files.
//!
//! Only the simple subset of the menu specification used by these applications is supported:
//! `<Include>` rules containing `<Category>`, `<Filename>` and `<Or>`, `<Directory>` and
//! `<DirectoryDir>`. Nested submenus are flattened into a single category.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::model::application_category::{ApplicationCategory, CategoryIcon};

/// A category menu found in a merged `.menu` file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuDefinition {
    /// Value of the `<Name>` element, used as a unique key and fallback display name.
    pub name: String,
    /// Value of the `<Directory>` element, name of the `.directory` file.
    pub directory: Option<String>,
    /// Additional directories in which the `.directory` file is searched for.
    pub directory_dirs: Vec<PathBuf>,
    /// Desktop entry categories included in this menu.
    pub categories: Vec<String>,
    /// Desktop file ids included in this menu.
    pub filenames: Vec<String>,
}

impl MenuDefinition {
    fn merge(&mut self, other: MenuDefinition) {
        if self.directory.is_none() {
            self.directory = other.directory;
        }
        extend_unique(&mut self.directory_dirs, other.directory_dirs);
        extend_unique(&mut self.categories, other.categories);
        extend_unique(&mut self.filenames, other.filenames);
    }
}

/// Loads custom categories from all `applications-merged` menu directories.
pub fn load_custom_categories() -> Vec<ApplicationCategory> {
    let mut definitions: Vec<MenuDefinition> = Vec::new();

    for path in merged_menu_files() {
        let Ok(content) = fs::read_to_string(&path) else {
            log::warn!("Failed to read menu file {}", path.display());
            continue;
        };
        let base_dir = path.parent().unwrap_or(Path::new("/"));

        for mut definition in parse_menu(&content) {
            definition.directory_dirs = definition
                .directory_dirs
                .into_iter()
                .map(|dir| base_dir.join(dir))
                .collect();

            match definitions.iter_mut().find(|d| d.name == definition.name) {
                Some(existing) => existing.merge(definition),
                None => definitions.push(definition),
            }
        }
    }

    definitions
        .into_iter()
        .filter(|d| !d.categories.is_empty() || !d.filenames.is_empty())
        .map(into_category)
        .collect()
}

/// Parses the content of a merged menu file and returns the category menus found in it.
///
/// The first menu below the root which has an `<Include>` or `<Directory>` element is treated
/// as a category, and the include rules of all of its submenus are merged into it.
pub fn parse_menu(content: &str) -> Vec<MenuDefinition> {
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let document = match roxmltree::Document::parse_with_options(content, options) {
        Ok(document) => document,
        Err(e) => {
            log::warn!("Failed to parse menu file: {e}");
            return Vec::new();
        }
    };

    let root = document.root_element();
    if !root.has_tag_name("Menu") {
        return Vec::new();
    }

    let mut definitions = Vec::new();
    let directory_dirs = directory_dirs_of(root);
    for menu in child_elements(root, "Menu") {
        find_category_menus(menu, &directory_dirs, &mut definitions);
    }
    definitions
}

fn find_category_menus(
    menu: roxmltree::Node,
    parent_directory_dirs: &[PathBuf],
    definitions: &mut Vec<MenuDefinition>,
) {
    let mut directory_dirs = parent_directory_dirs.to_vec();
    extend_unique(&mut directory_dirs, directory_dirs_of(menu));

    let is_category = child_elements(menu, "Include").next().is_some()
        || child_elements(menu, "Directory").next().is_some();

    if !is_category {
        for submenu in child_elements(menu, "Menu") {
            find_category_menus(submenu, &directory_dirs, definitions);
        }
        return;
    }

    let Some(name) = child_text(menu, "Name") else {
        return;
    };

    let mut definition = MenuDefinition {
        name,
        // the last <Directory> element takes precedence
        directory: child_elements(menu, "Directory").filter_map(text_of).last(),
        directory_dirs,
        ..Default::default()
    };
    collect_includes(menu, &mut definition);
    definitions.push(definition);
}

/// Collects include rules of the menu and all of its submenus.
fn collect_includes(menu: roxmltree::Node, definition: &mut MenuDefinition) {
    for include in child_elements(menu, "Include") {
        collect_rules(include, definition);
    }
    for submenu in child_elements(menu, "Menu") {
        extend_unique(&mut definition.directory_dirs, directory_dirs_of(submenu));
        collect_includes(submenu, definition);
    }
}

/// Collects `<Category>` and `<Filename>` rules, also from nested `<Or>` elements.
fn collect_rules(node: roxmltree::Node, definition: &mut MenuDefinition) {
    for child in node.children().filter(roxmltree::Node::is_element) {
        match child.tag_name().name() {
            "Category" => {
                if let Some(category) = text_of(child) {
                    extend_unique(&mut definition.categories, [category]);
                }
            }
            "Filename" => {
                if let Some(filename) = text_of(child) {
                    extend_unique(&mut definition.filenames, [filename]);
                }
            }
            "Or" => collect_rules(child, definition),
            _ => (),
        }
    }
}

fn into_category(definition: MenuDefinition) -> ApplicationCategory {
    let directory_entry = definition
        .directory
        .as_deref()
        .and_then(|directory| find_directory_file(directory, &definition.directory_dirs))
        .and_then(|path| read_directory_file(&path));

    let (display_name, icon) = match directory_entry {
        Some((name, icon)) => (name.unwrap_or_else(|| definition.name.clone()), icon),
        None => (definition.name.clone(), None),
    };

    ApplicationCategory {
        display_name: display_name.into(),
        icon: icon.unwrap_or(CategoryIcon::Named("folder-symbolic".into())),
        mime_name: definition.name.into(),
        permanent: false,
        include_categories: definition.categories,
        include_filenames: definition.filenames,
    }
}

/// Returns the localized name and the icon of a `.directory` file.
fn read_directory_file(path: &Path) -> Option<(Option<String>, Option<CategoryIcon>)> {
    let content = fs::read_to_string(path).ok()?;
    let locale = std::env::var("LANG")
        .ok()
        .and_then(|l| l.split(".").next().map(str::to_string));
    let (name, icon) = parse_directory_entry(&content, locale.as_deref());

    let icon = icon.map(|icon| {
        if Path::new(&icon).is_absolute() {
            CategoryIcon::Path(PathBuf::from(icon))
        } else {
            CategoryIcon::Named(icon)
        }
    });

    Some((name, icon))
}

/// Parses the `Name` (localized when available) and `Icon` keys of a `.directory` file.
///
/// `freedesktop-desktop-entry` only accepts files with the `.desktop` extension,
/// so the few keys needed are parsed here.
fn parse_directory_entry(content: &str, locale: Option<&str>) -> (Option<String>, Option<String>) {
    // e.g. for `sk_SK@euro` try `sk_SK@euro`, `sk_SK` and `sk`
    let locale = locale.unwrap_or_default();
    let without_modifier = locale.split('@').next().unwrap_or_default();
    let language = without_modifier.split('_').next().unwrap_or_default();
    let localized_keys: Vec<String> = [locale, without_modifier, language]
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(|l| format!("Name[{l}]"))
        .collect();

    let mut in_desktop_entry = false;
    let mut name = None;
    let mut localized_name: Option<(usize, String)> = None;
    let mut icon = None;

    for line in content.lines().map(str::trim) {
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        if value.is_empty() {
            continue;
        }

        if key == "Name" {
            name = Some(value.to_string());
        } else if key == "Icon" {
            icon = Some(value.to_string());
        } else if let Some(priority) = localized_keys.iter().position(|k| k == key) {
            if localized_name.as_ref().is_none_or(|(p, _)| priority < *p) {
                localized_name = Some((priority, value.to_string()));
            }
        }
    }

    (localized_name.map(|(_, name)| name).or(name), icon)
}

fn find_directory_file(directory: &str, directory_dirs: &[PathBuf]) -> Option<PathBuf> {
    let path = Path::new(directory);
    if path.is_absolute() {
        return path.is_file().then(|| path.to_path_buf());
    }

    // directories from the menu file take precedence over the default ones
    directory_dirs
        .iter()
        .rev()
        .cloned()
        .chain(data_dirs().into_iter().map(|dir| dir.join("desktop-directories")))
        .map(|dir| dir.join(path))
        .find(|candidate| candidate.is_file())
}

/// Returns paths of all merged menu files, ordered by priority. A file in a directory with
/// a higher priority shadows a file with the same name in a directory with a lower priority.
fn merged_menu_files() -> Vec<PathBuf> {
    let mut seen_names = Vec::new();
    let mut files = Vec::new();

    for dir in config_dirs() {
        let Ok(entries) = fs::read_dir(dir.join("menus/applications-merged")) else {
            continue;
        };

        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "menu"))
            .collect();
        paths.sort();

        for path in paths {
            let name = path.file_name().map(|name| name.to_os_string());
            if !seen_names.contains(&name) {
                seen_names.push(name);
                files.push(path);
            }
        }
    }

    files
}

/// `$XDG_CONFIG_HOME` followed by `$XDG_CONFIG_DIRS`.
fn config_dirs() -> Vec<PathBuf> {
    xdg_dirs("XDG_CONFIG_HOME", ".config", "XDG_CONFIG_DIRS", "/etc/xdg")
}

/// `$XDG_DATA_HOME` followed by `$XDG_DATA_DIRS`.
fn data_dirs() -> Vec<PathBuf> {
    xdg_dirs(
        "XDG_DATA_HOME",
        ".local/share",
        "XDG_DATA_DIRS",
        "/usr/local/share:/usr/share",
    )
}

fn xdg_dirs(home_var: &str, home_default: &str, dirs_var: &str, dirs_default: &str) -> Vec<PathBuf> {
    let home_dir = std::env::var_os(home_var)
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(home_default)));

    let dirs = std::env::var(dirs_var)
        .ok()
        .filter(|dirs| !dirs.is_empty())
        .unwrap_or_else(|| dirs_default.to_string());

    home_dir
        .into_iter()
        .chain(dirs.split(':').filter(|dir| !dir.is_empty()).map(PathBuf::from))
        .collect()
}

fn directory_dirs_of(menu: roxmltree::Node) -> Vec<PathBuf> {
    child_elements(menu, "DirectoryDir")
        .filter_map(text_of)
        .map(PathBuf::from)
        .collect()
}

fn child_elements<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    tag_name: &'static str,
) -> impl Iterator<Item = roxmltree::Node<'a, 'input>> {
    node.children().filter(move |child| child.has_tag_name(tag_name))
}

fn child_text(node: roxmltree::Node, tag_name: &'static str) -> Option<String> {
    child_elements(node, tag_name).find_map(text_of)
}

fn text_of(node: roxmltree::Node) -> Option<String> {
    node.text()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn extend_unique<T: PartialEq>(target: &mut Vec<T>, items: impl IntoIterator<Item = T>) {
    for item in items {
        if !target.contains(&item) {
            target.push(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CITRIX_MENU: &str = r#"<!DOCTYPE Menu PUBLIC "-//freedesktop//DTD Menu 1.0//EN"
"http://www.freedesktop.org/standards/menu-spec/menu-1.0.dtd">
<!-- This file was created by the Citrix StoreFront sub-system. -->
<Menu>
 <Name>Applications</Name>
 <Menu>
  <Name>Citrix Subscriptions</Name>
  <Include>
  <Category>Citrix-subscribed</Category>
  </Include>
 </Menu>
</Menu>"#;

    const WINE_MENU: &str = r#"<!DOCTYPE Menu
  PUBLIC '-//freedesktop//DTD Menu 1.0//EN'
  'http://standards.freedesktop.org/menu-spec/menu-1.0.dtd'>
<Menu>
	<Name>Applications</Name>
	<Menu>
		<Name>wine-wine</Name>
		<Directory>Wine.directory</Directory>
		<Include>
			<Category>X-Wine</Category>
		</Include>
	</Menu>
</Menu>"#;

    const WINE_PROGRAM_MENU: &str = r#"<!DOCTYPE Menu PUBLIC "-//freedesktop//DTD Menu 1.0//EN"
"http://www.freedesktop.org/standards/menu-spec/menu-1.0.dtd">
<Menu>
  <Name>Applications</Name>
  <Menu>
    <Name>wine-wine</Name>
    <Directory>wine-wine.directory</Directory>
    <Menu>
      <Name>wine-Programs</Name>
      <Directory>wine-Programs.directory</Directory>
      <Menu>
        <Name>wine-Programs-Notepad++</Name>
        <Directory>wine-Programs-Notepad++.directory</Directory>
        <Include>
          <Filename>wine-Programs-Notepad++-Notepad++.desktop</Filename>
        </Include>
      </Menu>
    </Menu>
  </Menu>
</Menu>"#;

    const CISCO_MENU: &str = r#"<!DOCTYPE Menu PUBLIC "-//freedesktop//DTD Menu 1.0//EN"
"http://www.freedesktop.org/standards/menu-spec/menu-1.0.dtd">

<Menu>
  <MergeFile type="path">cisco-secure-client-dart.menu</MergeFile>
  <AppDir>/usr/share/applications</AppDir>
  <DirectoryDir>/usr/share/desktop-directories</DirectoryDir>
  <Name>Applications</Name>
  <Menu>
    <Name>Internet</Name>
    <Menu>
      <Name>Cisco Secure Client</Name>
      <Directory>cisco-secure-client.directory</Directory>
      <Include>
        <Filename>com.cisco.secureclient.gui.desktop</Filename>
      </Include>
    </Menu>
  </Menu>
</Menu>"#;

    #[test]
    fn parses_category_include() {
        assert_eq!(
            parse_menu(CITRIX_MENU),
            vec![MenuDefinition {
                name: "Citrix Subscriptions".into(),
                categories: vec!["Citrix-subscribed".into()],
                ..Default::default()
            }]
        );
    }

    #[test]
    fn parses_directory() {
        assert_eq!(
            parse_menu(WINE_MENU),
            vec![MenuDefinition {
                name: "wine-wine".into(),
                directory: Some("Wine.directory".into()),
                categories: vec!["X-Wine".into()],
                ..Default::default()
            }]
        );
    }

    #[test]
    fn flattens_nested_menus() {
        assert_eq!(
            parse_menu(WINE_PROGRAM_MENU),
            vec![MenuDefinition {
                name: "wine-wine".into(),
                directory: Some("wine-wine.directory".into()),
                filenames: vec!["wine-Programs-Notepad++-Notepad++.desktop".into()],
                ..Default::default()
            }]
        );
    }

    #[test]
    fn descends_into_menus_without_includes() {
        assert_eq!(
            parse_menu(CISCO_MENU),
            vec![MenuDefinition {
                name: "Cisco Secure Client".into(),
                directory: Some("cisco-secure-client.directory".into()),
                directory_dirs: vec!["/usr/share/desktop-directories".into()],
                filenames: vec!["com.cisco.secureclient.gui.desktop".into()],
                ..Default::default()
            }]
        );
    }

    #[test]
    fn parses_or_rules() {
        let menu = r#"<Menu><Name>Applications</Name><Menu><Name>Custom</Name>
            <Include><Or><Category>A</Category><Filename>b.desktop</Filename></Or></Include>
            </Menu></Menu>"#;
        assert_eq!(
            parse_menu(menu),
            vec![MenuDefinition {
                name: "Custom".into(),
                categories: vec!["A".into()],
                filenames: vec!["b.desktop".into()],
                ..Default::default()
            }]
        );
    }

    #[test]
    fn parses_directory_entry() {
        let entry = "[Desktop Entry]\nEncoding=UTF-8\nType=Directory\nName=Wine\n\
                     Name[sk]=Víno\nName[sk_SK]=Víno SK\nIcon=wine\n[Other]\nName=Other\n";
        assert_eq!(
            parse_directory_entry(entry, Some("sk_SK")),
            (Some("Víno SK".into()), Some("wine".into()))
        );
        assert_eq!(
            parse_directory_entry(entry, Some("sk_CZ@euro")),
            (Some("Víno".into()), Some("wine".into()))
        );
        assert_eq!(
            parse_directory_entry(entry, None),
            (Some("Wine".into()), Some("wine".into()))
        );
        assert_eq!(parse_directory_entry("[Desktop Entry]\nType=Directory", None), (None, None));
    }

    #[test]
    fn ignores_empty_and_invalid_files() {
        assert!(parse_menu("").is_empty());
        assert!(parse_menu("<Menu>").is_empty());
        assert!(parse_menu("<Menu><Name>Applications</Name></Menu>").is_empty());
    }

    #[test]
    fn merges_definitions_with_same_name() {
        let mut definitions = parse_menu(WINE_MENU);
        definitions[0].merge(parse_menu(WINE_PROGRAM_MENU).remove(0));
        assert_eq!(
            definitions,
            vec![MenuDefinition {
                name: "wine-wine".into(),
                directory: Some("Wine.directory".into()),
                categories: vec!["X-Wine".into()],
                filenames: vec!["wine-Programs-Notepad++-Notepad++.desktop".into()],
                ..Default::default()
            }]
        );
    }
}
