use std::{collections::HashMap, time::Duration};

use log::info;
use tokio::{
    select,
    time::{interval, Interval, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;

use crate::player::Player;

#[derive(Debug)]
pub(crate) struct GameRoom {
    id: String,
    _players: HashMap<String, Player>,
    cancellation_token: CancellationToken,
}

impl GameRoom {
    pub fn _new(id: String, cancellation_token: CancellationToken) -> Self {
        Self {
            id,
            _players: HashMap::new(),
            cancellation_token,
        }
    }

    pub async fn start(&self) {
        let cancellation_token = self.cancellation_token.clone();
        let id = self.id.clone();
        let mut interval = self.get_interval();

        loop {
            select! {
                biased;

                _ = cancellation_token.cancelled() => {
                    info!("Room {} has been cancelled. Exiting.", id);
                    break;
                }

                _instant = interval.tick() => {
                    // TODO: update the world state.
                }
            }
        }
    }

    #[inline]
    fn get_interval(&self) -> Interval {
        let mut interval = interval(Duration::from_millis(2));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        interval
    }
}
