use std::time::Duration;

use dashmap::DashMap;
use log::{debug, warn};
use tokio::{
    select,
    sync::RwLock,
    time::{interval, Interval, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;

use crate::{game_room_status::GameRoomStatus, player::Player, GameStateConfig};

pub struct HasGameRoomStarted(bool);

impl Into<bool> for HasGameRoomStarted {
    fn into(self) -> bool {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct GameRoom {
    config: GameStateConfig,
    players: DashMap<String, Player>,
    game_room_status: RwLock<GameRoomStatus>,
    cancellation_token: CancellationToken,
}

impl GameRoom {
    pub fn new(config: GameStateConfig, cancellation_token: CancellationToken) -> Self {
        Self {
            config,
            players: DashMap::new(),
            game_room_status: Default::default(),
            cancellation_token,
        }
    }

    pub async fn add_player(&self, nick: String, player: Player) -> HasGameRoomStarted {
        let mut game_status = self.game_room_status.write().await;

        if let GameRoomStatus::Running = game_status.clone() {
            warn!(
                "Tried to add player {} to a game that's already started.",
                nick
            );

            return HasGameRoomStarted(true);
        }

        self.players.insert(nick, player);

        if self.players.len() >= self.config.number_of_players_in_game_room {
            HasGameRoomStarted(false)
        } else {
            *game_status = GameRoomStatus::Running;
            self.start().await;
            HasGameRoomStarted(true)
        }
    }

    pub async fn start(&self) {
        let cancellation_token = self.cancellation_token.clone();
        let mut interval = self.get_interval();

        tokio::spawn(async move {
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
        });
    }

    pub async fn get_state(&self) -> GameRoomStatus {
        self.game_room_status.read().await.clone()
    }

    #[inline]
    fn get_interval(&self) -> Interval {
        let mut interval = interval(Duration::from_millis(2));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        interval
    }
}
