use std::{fs, path::Path, str::Split};

use freedesktop_desktop_entry::{default_paths, get_languages_from_env, Iter, PathSource, DesktopEntry};

pub fn get_applications_list() -> Vec<String> {
    const XDG_DATA_DIRS: &str = env!("XDG_DATA_DIRS");
    let applications_locations: Split<&str> = XDG_DATA_DIRS.split(":");
    for x in applications_locations {
        dbg!(x);

        let folder_content = fs::read_dir(x.to_string() + "/applications");
        match folder_content {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if path.ends_with(".desktop") {
                            }
                            println!("{:?}", path);
                        }
                        Err(e) => eprintln!("Error reading entry: {:?}", e),
                    }
                }
            }
            Err(_) => (),
        }
    }

    vec!["Hello".to_string()]
}

pub fn get_apps() -> Vec<DesktopEntry> {
    let locales = get_languages_from_env();

    let entries = Iter::new(default_paths())
        .entries(Some(&locales))
        .collect::<Vec<_>>();

    entries
}
