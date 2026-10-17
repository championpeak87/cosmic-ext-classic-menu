use cosmic::iced::{Subscription, stream};
use futures::{SinkExt, channel::mpsc::Sender};
use tokio::sync::mpsc;

use crate::{applet::Message, dbus::interface::AppletSignalsService};

pub mod interface;

/// Well-known name of the applet's D-Bus service.
pub const SERVICE_NAME: &str = "com.championpeak87.CosmicExtClassicMenu";
/// Object path of the applet's D-Bus service.
pub const OBJECT_PATH: &str = "/com/championpeak87/CosmicExtClassicMenu";

/// Client side of [`AppletSignalsService`].
#[zbus::proxy(
    interface = "com.championpeak87.CosmicExtClassicMenu",
    default_service = "com.championpeak87.CosmicExtClassicMenu",
    default_path = "/com/championpeak87/CosmicExtClassicMenu"
)]
trait AppletSignals {
    fn toggle_popup_signal(&self) -> zbus::Result<()>;
}

/// Opens the menu of the running applet, or closes it if it is open.
pub fn toggle_popup() -> zbus::Result<()> {
    let connection = zbus::blocking::Connection::session()?;
    AppletSignalsProxyBlocking::new(&connection)?.toggle_popup_signal()
}

pub fn dbus_service_subscription() -> Subscription<Message> {
    Subscription::run(|| {
        stream::channel(100, |mut output: Sender<Message>| async move {
            let (tx, mut rx) = mpsc::unbounded_channel();

            // Register the D-Bus service with the channel sender
            let service = AppletSignalsService { tx };
            
            match zbus::connection::Builder::session() {
                Ok(builder) => {
                    match builder
                        .name(SERVICE_NAME)
                        .and_then(|b| b.serve_at(OBJECT_PATH, service))
                        .unwrap()
                        .build()
                        .await
                    {
                        Ok(_conn) => {
                            // Keep connection alive and listen for messages
                            while let Some(msg) = rx.recv().await {
                                let _ = output.send(msg).await;
                            }
                        }
                        Err(e) => log::error!("Failed to setup D-Bus service: {}", e),
                    }
                }
                Err(e) => log::error!("Failed to create D-Bus builder: {}", e),
            }
        })
    })
}