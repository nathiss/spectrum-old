use std::collections::HashMap;

use crate::player::Player;

#[derive(Debug)]
pub(crate) struct GameRoom {
    _players: HashMap<String, Player>,
}

impl GameRoom {
    pub fn _new() -> Self {
        Self {
            _players: HashMap::new(),
        }
    }
}
