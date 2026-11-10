// SPDX-License-Identifier: GPL-3.0-only

//! Search results of pop-launcher, the service behind COSMIC's launcher.
//!
//! Its plugins add what the menu's own search does not find: a calculator,
//! files, open windows, web searches, commands and scripts. Applications
//! are left to the menu's own search.
//!
//! pop-launcher runs as a child process taking JSON requests on its stdin
//! and answering with one JSON response per line on its stdout. In a
//! Flatpak, it runs on the host, where its plugins and the apps they open
//! are.

use std::path::PathBuf;
use std::time::Duration;

use cosmic::iced::{Subscription, stream};
use futures::SinkExt;
use futures::channel::mpsc::Sender;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::mpsc;

/// Icon of the plugin listing applications, which marks its results.
const DESKTOP_ENTRIES_ICON: &str = "new-window-symbolic";

/// How long to wait before starting pop-launcher again once it exits.
const RESTART_DELAY: Duration = Duration::from_secs(5);

/// A request to pop-launcher.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Request {
    /// Searches for the query, answered by [`Response::Update`].
    Search(String),
    /// Activates the result of this id from the last update.
    Activate(u32),
}

/// An icon, by name or path, or by the MIME type of a file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum IconSource {
    Name(String),
    Mime(String),
}

/// One search result of pop-launcher.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SearchResult {
    /// Identifies the result in [`Request::Activate`].
    pub id: u32,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub icon: Option<IconSource>,
    /// Icon of the plugin the result is from.
    #[serde(default)]
    pub category_icon: Option<IconSource>,
}

impl SearchResult {
    /// Whether the result is an application, which the menu finds itself.
    pub fn is_application(&self) -> bool {
        self.plugin_icon() == Some(DESKTOP_ENTRIES_ICON)
    }

    /// The kind of result, told by the icon of the plugin it is from.
    pub fn group(&self) -> ResultGroup {
        match self.plugin_icon() {
            Some("x-office-spreadsheet") => ResultGroup::Calculator,
            Some("focus-windows-symbolic") => ResultGroup::Windows,
            // Files, find and recent.
            Some("system-file-manager") => ResultGroup::Files,
            Some("system-search") => ResultGroup::Web,
            // Terminal commands and scripts.
            Some("utilities-terminal") => ResultGroup::Commands,
            Some("multimedia-volume-control") => ResultGroup::Sound,
            _ => ResultGroup::Other,
        }
    }

    fn plugin_icon(&self) -> Option<&str> {
        match &self.category_icon {
            Some(IconSource::Name(name)) => Some(name),
            _ => None,
        }
    }
}

/// A kind of result, shown under a heading of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultGroup {
    Calculator,
    Windows,
    Files,
    Web,
    Commands,
    Sound,
    Other,
}

/// `results` by their group, the groups in the order pop-launcher ranks
/// their best results, and each result in its own order within its group.
pub fn grouped(results: &[SearchResult]) -> Vec<(ResultGroup, Vec<&SearchResult>)> {
    let mut groups: Vec<(ResultGroup, Vec<&SearchResult>)> = Vec::new();
    for result in results {
        let group = result.group();
        match groups.iter_mut().find(|(other, _)| *other == group) {
            Some((_, members)) => members.push(result),
            None => groups.push((group, vec![result])),
        }
    }
    groups
}

/// A response of pop-launcher.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Response {
    /// The activated result is done, so the menu closes.
    Close,
    /// The search results.
    Update(Vec<SearchResult>),
    /// Replaces the query, e.g. completing a path.
    Fill(String),
    /// An application to launch.
    DesktopEntry { path: PathBuf },
    /// Options for a result, which the menu does not offer.
    Context { id: u32 },
}

/// What the subscription reports.
#[derive(Debug, Clone)]
pub enum Event {
    /// pop-launcher runs and takes requests sent here.
    Started(mpsc::UnboundedSender<Request>),
    Response(Response),
    /// pop-launcher stopped, so requests are dropped until it starts again.
    Stopped,
}

/// Results worth showing next to the menu's own: all but applications.
pub fn non_application_results(results: Vec<SearchResult>) -> Vec<SearchResult> {
    results
        .into_iter()
        .filter(|result| !result.is_application())
        .collect()
}

/// Runs pop-launcher for as long as the applet does.
pub fn subscription() -> Subscription<Event> {
    Subscription::run(|| {
        stream::channel(16, |mut output: Sender<Event>| async move {
            loop {
                match spawn() {
                    Ok((child, stdin, stdout)) => {
                        serve(child, stdin, stdout, &mut output).await;
                        let _ = output.send(Event::Stopped).await;
                        log::warn!("pop-launcher stopped, starting it again");
                    }
                    Err(err) => {
                        // Not installed, most likely: searching goes on
                        // without it.
                        log::warn!("Failed to start pop-launcher: {err}");
                        std::future::pending::<()>().await;
                    }
                }
                tokio::time::sleep(RESTART_DELAY).await;
            }
        })
    })
}

/// Starts pop-launcher, on the host when in a Flatpak.
fn spawn() -> std::io::Result<(Child, ChildStdin, ChildStdout)> {
    let mut command = if std::env::var_os("FLATPAK_ID").is_some() {
        let mut command = Command::new("flatpak-spawn");
        command.args(["--host", "pop-launcher"]);
        command
    } else {
        Command::new("pop-launcher")
    };
    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let stdin = child.stdin.take().expect("stdin is piped");
    let stdout = child.stdout.take().expect("stdout is piped");
    Ok((child, stdin, stdout))
}

/// Passes requests to pop-launcher and its responses on, until it exits.
async fn serve(
    mut child: Child,
    mut stdin: ChildStdin,
    stdout: ChildStdout,
    output: &mut Sender<Event>,
) {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    if output.send(Event::Started(requests_tx)).await.is_err() {
        return;
    }
    let mut lines = BufReader::new(stdout).lines();

    loop {
        tokio::select! {
            request = requests.recv() => {
                let Some(request) = request else { break };
                let Ok(mut line) = serde_json::to_string(&request) else { continue };
                line.push('\n');
                if stdin.write_all(line.as_bytes()).await.is_err()
                    || stdin.flush().await.is_err()
                {
                    break;
                }
            }
            line = lines.next_line() => {
                let Ok(Some(line)) = line else { break };
                match serde_json::from_str::<Response>(&line) {
                    Ok(response) => {
                        if output.send(Event::Response(response)).await.is_err() {
                            break;
                        }
                    }
                    Err(err) => log::debug!("Ignored pop-launcher response {line}: {err}"),
                }
            }
        }
    }

    let _ = child.kill().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_written_as_pop_launcher_reads_them() {
        let search = serde_json::to_string(&Request::Search("= 2+3".to_owned())).unwrap();
        assert_eq!(search, r#"{"Search":"= 2+3"}"#);
        assert_eq!(serde_json::to_string(&Request::Activate(4)).unwrap(), r#"{"Activate":4}"#);
    }

    #[test]
    fn updates_are_read_with_or_without_icons() {
        // As pop-launcher answers "= 2+3" and "t: ls".
        let line = r#"{"Update":[{"id":0,"name":"5","description":"","icon":{"Name":"accessories-calculator"},"category_icon":{"Name":"x-office-spreadsheet"}},{"id":1,"name":"t: ls","description":"run command in terminal","category_icon":{"Name":"utilities-terminal"}}]}"#;
        let Response::Update(results) = serde_json::from_str(line).unwrap() else {
            panic!("not an update");
        };

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].icon, Some(IconSource::Name("accessories-calculator".into())));
        assert_eq!(results[1].icon, None);
        assert_eq!(results[1].description, "run command in terminal");
    }

    #[test]
    fn other_responses_are_read() {
        assert_eq!(serde_json::from_str::<Response>(r#""Close""#).unwrap(), Response::Close);
        assert_eq!(
            serde_json::from_str::<Response>(r#"{"Fill":"~/Documents/"}"#).unwrap(),
            Response::Fill("~/Documents/".into())
        );
        let entry = r#"{"DesktopEntry":{"path":"/usr/share/applications/firefox.desktop","gpu_preference":"Default","action_name":null}}"#;
        assert_eq!(
            serde_json::from_str::<Response>(entry).unwrap(),
            Response::DesktopEntry { path: "/usr/share/applications/firefox.desktop".into() }
        );
    }

    fn from_plugin(name: &str, plugin_icon: &str) -> SearchResult {
        SearchResult {
            id: 0,
            name: name.to_owned(),
            description: String::new(),
            icon: None,
            category_icon: Some(IconSource::Name(plugin_icon.to_owned())),
        }
    }

    #[test]
    fn results_are_grouped_in_the_order_they_come() {
        let results = vec![
            from_plugin("Firefox", "focus-windows-symbolic"),
            from_plugin("ddg fire", "system-search"),
            from_plugin("Files", "focus-windows-symbolic"),
            from_plugin("fire.txt", "system-file-manager"),
        ];

        let groups = grouped(&results);
        let names: Vec<(ResultGroup, Vec<&str>)> = groups
            .iter()
            .map(|(group, members)| (*group, members.iter().map(|r| r.name.as_str()).collect()))
            .collect();
        assert_eq!(
            names,
            vec![
                (ResultGroup::Windows, vec!["Firefox", "Files"]),
                (ResultGroup::Web, vec!["ddg fire"]),
                (ResultGroup::Files, vec!["fire.txt"]),
            ]
        );
    }

    #[test]
    fn unknown_plugins_are_other_results() {
        assert_eq!(from_plugin("x", "some-plugin").group(), ResultGroup::Other);
        assert_eq!(from_plugin("5", "x-office-spreadsheet").group(), ResultGroup::Calculator);
    }

    #[test]
    fn applications_are_left_to_the_menu() {
        let results = vec![
            from_plugin("Firefox", DESKTOP_ENTRIES_ICON),
            from_plugin("Firefox — Mozilla", "focus-windows-symbolic"),
        ];

        let kept = non_application_results(results);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].name, "Firefox — Mozilla");
    }
}
