use io::*;

use mio;
use ws::*;
use serde_json;

pub struct ConfigServer {
    client: Sender,
    server: mio::Sender<(NetEvent, Sender)>,
}

impl ConfigServer {
    pub fn new(client: Sender, server: mio::Sender<(NetEvent, Sender)>) -> Self {
        ConfigServer { client: client, server: server }
    }
}

impl Handler for ConfigServer {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        self.server.send((NetEvent::RequestCluster, self.client.clone())).unwrap();
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let msg = msg.as_text().unwrap();
        let event = serde_json::from_str(msg).unwrap();
        self.server.send((event, self.client.clone())).unwrap();
        Ok(())
    }
}
