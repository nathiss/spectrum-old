use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use log::debug;
use spectrum_packet::model::ClientMessage;
use tokio::{
    select,
    sync::{mpsc::UnboundedReceiver, RwLock},
};
use tokio_util::sync::CancellationToken;

use crate::Client;

use super::client_map_key::ClientMapKey;

pub(super) async fn create_receive_task(
    key: ClientMapKey,
    mut packet_rx: UnboundedReceiver<ClientMessage>,
    addr: SocketAddr,
    new_clients: Arc<RwLock<HashMap<ClientMapKey, Client>>>,
    cancellation_token: CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            select! {
                biased;

                _ = cancellation_token.cancelled() => {
                    debug!("The sever has been cancelled. Receiving task for {} will now exit.", addr);

                    new_clients.write().await.remove(&key);
                    break;
                }

                message = packet_rx.recv() => {
                    if let None = message {
                        debug!("Underlying Connection was closed. Receiving task for {} will now exit.", addr);
                        break;
                    }

                    // TODO: handle message
                }
            }
        }
    });
}
