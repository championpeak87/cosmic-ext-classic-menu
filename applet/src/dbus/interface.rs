// SPDX-License-Identifier: GPL-3.0-only

use tokio::sync::mpsc::UnboundedSender;
use zbus::interface;

use crate::applet::Message;

pub struct AppletSignalsService {
    pub tx: UnboundedSender<Message>,
}

#[interface(name = "com.championpeak87.CosmicExtClassicMenu")]
impl AppletSignalsService {
    fn toggle_popup_signal(&self) -> () {
        self.tx.send(Message::SuperKeyPressed).unwrap();
    }
}
