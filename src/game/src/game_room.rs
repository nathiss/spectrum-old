use std::time::Duration;

use dashmap::DashMap;
use log::{debug, error, warn};
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

/// This struct represents a single game room and all its associated content.
#[derive(Debug)]
pub(crate) struct GameRoom {
    config: GameStateConfig,
    players: DashMap<String, Player>,
    game_room_status: RwLock<GameRoomStatus>,
    cancellation_token: CancellationToken,
}

impl GameRoom {
    /// This method is used to construct a new game room.
    ///
    /// # Arguments
    ///
    /// * `config` - A configuration for the game room and all its internal components.
    /// * `cancellation_token` - A server's cancellation token. If canceled it will cause all internal asynchronous
    ///                          tasks to complete immediately.
    pub fn new(config: GameStateConfig, cancellation_token: CancellationToken) -> Self {
        let maximum_number_of_players = config.number_of_players_in_game_room;

        Self {
            config,
            players: DashMap::with_capacity(maximum_number_of_players),
            game_room_status: Default::default(),
            cancellation_token,
        }
    }

    /// This method is used to add a new player into the game room.
    ///
    /// This method can cause the game to start if the maximum number of players has been reached. Before the player is
    /// actually added into the room, this method first checks the room's state. If it's not `GameRoomStatus::Waiting`
    /// then the method returns immediately with `HasGameRoomStarted(true)` as the return value.
    ///
    /// # Arguments
    ///
    /// * `nick` - A nick for the new player.
    /// * `player` - A context object contains all player's associated data.
    ///
    /// # Returns
    ///
    /// An indication of whether the operation caused the game to start is returned.
    pub async fn add_player(&self, nick: String, player: Player) -> HasGameRoomStarted {
        // This lock guard ensures that only one thread can access this method at any given moment.
        let mut game_status = self.game_room_status.write().await;

        if *game_status != GameRoomStatus::Running {
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

    /// This method is used to start the game omitting all constrains.
    ///
    /// It spawns a new asynchronous task which handles all communication channels with all players that belong to this
    /// game room. It can be cancelled via the cancellation token passed to the `new()` method as an argument. This
    /// method can only be called once, when the room's state is `GameRoomState::Waiting`. Any other calls will result
    /// in a panic.
    pub async fn start(&self) {
        let mut game_room_state = self.game_room_status.write().await;
        if *game_room_state != GameRoomStatus::Waiting {
            error!("GameRoom::start() has been called while the game is already running.");
            panic!("GameRoom::start() can only be called once.");
        }

        *game_room_state = GameRoomStatus::Running;

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
