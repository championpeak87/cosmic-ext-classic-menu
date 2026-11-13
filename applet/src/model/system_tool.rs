use std::process;

/// Starts the first system monitor found, preferring COSMIC's.
///
/// A shell script so that it looks where the monitors are, on the host even
/// from within the Flatpak sandbox, and notices ones installed later.
const FIRST_SYSTEM_MONITOR: &str = concat!(
    "program() { command -v \"$1\" >/dev/null && exec \"$1\"; }\n",
    "flatpak_app() { flatpak info \"$1\" >/dev/null 2>&1 && exec flatpak run \"$1\"; }\n",
    "program cosmic-monitor\n",
    "program observatory\n",
    "program gnome-system-monitor\n",
    "program plasma-systemmonitor\n",
    "program missioncenter\n",
    "program resources\n",
    "program mate-system-monitor\n",
    "program xfce4-taskmanager\n",
    "program lxtask\n",
    "flatpak_app net.nokyan.Resources\n",
    "flatpak_app io.missioncenter.MissionCenter\n",
    "flatpak_app org.gnome.SystemMonitor\n",
    "echo 'No system monitor found' >&2\n",
    "exit 127\n",
);

#[derive(Clone, Debug, PartialEq)]
pub struct SystemTool {
    exec: &'static str,
}

/// Runs a desktop entry's `exec` line detached from the applet.
///
/// Not `cosmic_launch::spawn`, which starts a systemd unit on the host: from
/// the Flatpak sandbox that unit would run `flatpak-spawn`, which is only
/// inside the sandbox, and the app would silently not open.
pub(crate) async fn spawn_exec(exec: String, terminal: bool, env_vars: Vec<(String, String)>) {
    let line = if terminal {
        format!("{} -e {exec}", crate::model::place::terminal_command())
    } else {
        exec
    };

    let args = match parse_exec(&line) {
        Some(args) => args,
        None => {
            log::error!("Cannot run '{line}'");
            return;
        }
    };

    let mut command = process::Command::new(&args[0]);
    command.args(&args[1..]).envs(env_vars);
    if cosmic_launch::process::spawn(command).await.is_none() {
        log::error!("Error launching '{line}'");
    }
}

/// Splits an `exec` line into a program and its arguments.
fn parse_exec(line: &str) -> Option<Vec<String>> {
    shlex::split(line).filter(|args| args.first().is_some_and(|program| !program.contains('=')))
}

impl SystemTool {
    pub const APPLET_SETTINGS: SystemTool = Self {
        exec: "cosmic-ext-classic-menu-settings",
    };
    pub const SYSTEM_SETTINGS: SystemTool = Self {
        exec: "cosmic-settings",
    };
    pub const SYSTEM_MONITOR: SystemTool = Self {
        exec: FIRST_SYSTEM_MONITOR,
    };
    pub const DISK_MANAGEMENT: SystemTool = Self {
        exec: "gnome-disks",
    };
    pub const APP_STORE: SystemTool = Self {
        exec: "cosmic-store",
    };

    /// Runs `exec`, a shell command line, on the host.
    pub const fn command(exec: &'static str) -> Self {
        Self { exec }
    }

    pub fn perform(&self) {
        if self == &SystemTool::APPLET_SETTINGS {
            self.handle_applet_settings();
            return;
        }

        self.handle_generic_tool();
    }

    fn handle_applet_settings(&self) {
        let env_vars: Vec<(String, String)> = std::env::vars().collect();

        // Spawn the asynchronous execution
        tokio::spawn(spawn_exec(
            SystemTool::APPLET_SETTINGS.exec.to_owned(),
            false,
            env_vars,
        ));
    }

    fn handle_generic_tool(&self) {
        let is_flatpak = std::env::var("FLATPAK_ID").is_ok();

        // Logic to determine the final command and arguments, centralizing Flatpak handling
        let (main_exec, args) = if is_flatpak {
            // For Flatpak, use `flatpak-spawn` with the `--host` argument
            (
                "flatpak-spawn",
                vec!["--host", "/bin/sh", "-l", "-c", self.exec],
            )
        } else {
            // For native, through a shell as `exec` may be a script
            ("/bin/sh", vec!["-c", self.exec])
        };

        // Execute the command and provide better error reporting
        if let Err(e) = process::Command::new(main_exec).args(args).spawn() {
            log::error!("Error launching tool '{}': {}", main_exec, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Output};

    /// Runs the script with only `installed` programs available.
    fn run_with(test: &str, installed: &[&str]) -> Output {
        let dir = std::env::temp_dir().join(format!(
            "classic-menu-monitor-{}-{test}",
            process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        for program in installed {
            let path = dir.join(program);
            fs::write(&path, format!("#!/bin/sh\necho {program} \"$@\"\n")).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        let output = Command::new("/bin/sh")
            .args(["-c", FIRST_SYSTEM_MONITOR])
            .env("PATH", &dir)
            .output()
            .unwrap();
        fs::remove_dir_all(&dir).unwrap();
        output
    }

    fn started(output: &Output) -> String {
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    #[test]
    fn parses_quoted_exec_lines() {
        assert_eq!(
            parse_exec("flatpak-spawn --host /bin/sh -c 'app --flag'").unwrap(),
            ["flatpak-spawn", "--host", "/bin/sh", "-c", "app --flag"]
        );
    }

    #[test]
    fn rejects_exec_lines_that_are_not_commands() {
        assert_eq!(parse_exec(""), None);
        assert_eq!(parse_exec("FOO=bar app"), None);
        assert_eq!(parse_exec("app 'unterminated"), None);
    }

    #[test]
    fn prefers_the_cosmic_monitor() {
        let output = run_with("cosmic", &["gnome-system-monitor", "cosmic-monitor"]);
        assert_eq!(started(&output), "cosmic-monitor");
    }

    #[test]
    fn falls_back_when_the_cosmic_monitor_is_missing() {
        let output = run_with("fallback", &["lxtask", "gnome-system-monitor"]);
        assert_eq!(started(&output), "gnome-system-monitor");
    }

    #[test]
    fn looks_for_flatpaks_after_programs() {
        // A `flatpak` where every app is installed.
        let output = run_with("flatpak", &["flatpak"]);
        assert_eq!(started(&output), "flatpak run net.nokyan.Resources");
    }

    #[test]
    fn reports_when_there_is_no_monitor() {
        let output = run_with("none", &[]);
        assert_eq!(output.status.code(), Some(127));
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).trim(),
            "No system monitor found"
        );
    }
}
