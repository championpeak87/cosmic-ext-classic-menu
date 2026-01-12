use cosmic::iced::{Subscription, stream};
use futures::SinkExt;
use tokio::sync::mpsc;

use crate::{applet::Message, dbus::interface::AppletSignalsService};

pub mod interface;

pub fn dbus_service_subscription() -> Subscription<Message> {
    Subscription::run(|| {
        stream::channel(100, |mut output| async move {
            let (tx, mut rx) = mpsc::unbounded_channel();

            // Register the D-Bus service with the channel sender
            let service = AppletSignalsService { tx };
            
            match zbus::connection::Builder::session() {
                Ok(builder) => {
                    match builder
                        .name("com.championpeak87.CosmicExtClassicMenu")
                        .and_then(|b| b.serve_at("/com/championpeak87/CosmicExtClassicMenu", service))
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