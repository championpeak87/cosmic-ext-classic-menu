use std::process;

#[derive(Clone, Debug, PartialEq)]
pub struct SystemTool {
    exec: &'static str,
}

impl SystemTool {
    pub const APPLET_SETTINGS: SystemTool = Self {
        exec: "cosmic-ext-classic-menu-settings",
    };
    pub const SYSTEM_SETTINGS: SystemTool = Self {
        exec: "cosmic-settings",
    };
    pub const SYSTEM_MONITOR: SystemTool = Self {
        exec: "gnome-system-monitor",
    };
    pub const DISK_MANAGEMENT: SystemTool = Self {
        exec: "gnome-disks",
    };

    pub fn perform(&self) {
        if self == &SystemTool::APPLET_SETTINGS {
            self.handle_applet_settings();
            return;
        }

        self.handle_generic_tool();
    }

    fn handle_applet_settings(&self) {
        let env_vars: Vec<(String, String)> = std::env::vars().collect();
        let app_id = Some("com.championpeak87.cosmic-ext-classic-menu.settings");

        // Spawn the asynchronous execution
        tokio::spawn(async move {
            cosmic::desktop::spawn_desktop_exec(
                SystemTool::APPLET_SETTINGS.exec,
                env_vars,
                app_id.as_deref(),
                false,
            )
            .await;
        });
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
            // For native, use the direct executable name
            (self.exec, vec![])
        };

        // Execute the command and provide better error reporting
        if let Err(e) = process::Command::new(main_exec).args(args).spawn() {
            log::error!("Error launching tool '{}': {}", main_exec, e);
        }
    }
}
