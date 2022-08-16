use std::{collections::HashMap, time::Duration};

use log::debug;
use tokio::{
    select,
    time::{interval, Interval, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;

use crate::{game_room_status::GameRoomStatus, player::Player};

#[derive(Debug)]
pub(crate) struct GameRoom {
    _players: HashMap<String, Player>,
    game_room_status: GameRoomStatus,
    cancellation_token: CancellationToken,
}

impl GameRoom {
    pub fn _new(cancellation_token: CancellationToken) -> Self {
        Self {
            _players: HashMap::new(),
            game_room_status: Default::default(),
            cancellation_token,
        }
    }

    pub async fn start(&self) {
        let cancellation_token = self.cancellation_token.clone();
        let mut interval = self.get_interval();

        loop {
            select! {
                biased;

                _ = cancellation_token.cancelled() => {
                    debug!("Game room has been cancelled. Exiting.");
                    break;
                }

                _instant = interval.tick() => {
                    // TODO: update the world state.
                }
            }
        }
    }

    pub fn get_state(&self) -> &GameRoomStatus {
        &self.game_room_status
    }

    #[inline]
    fn get_interval(&self) -> Interval {
        let mut interval = interval(Duration::from_millis(2));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        interval
    }
}
