# Technology stack

The game's server will be written in Rust.

## General

For managing custom settings this project will use [Serde](https://serde.rs/) crate for serialization and
deserialization of data. More specifically the [Serde YAML](https://github.com/dtolnay/serde-yaml) create for YAML
support.

The server must allow multiple connections from many endpoints and must be able to handle them. The Rust programming
language has a built-in support for asynchronous programming. It does not however provide any runtime, so it must be
provided as a third-party solution. A popular choice is a [Tokio](https://crates.io/crates/tokio) create.

Rust ecosystem provides a [log](https://crates.io/crates/log) create for abstracting logging interface from its
implementation. There are multiple logger implementations to choose from:

* [env_logger](https://crates.io/crates/env_logger) - Implements a logger that can be configured via environment
variables. *The most popular logger implementation on crates.io*
* [fern](https://crates.io/crates/fern) - Simple, efficient logging for Rust.

## Network-level

The game's underlying network protocol will be abstracted-away from the rest of the system, so it can be easily changed
without affecting the rest of the system.
When a new connected is established, then the socket is moved to the **network layer** which ensures that the connection
is initialized properly, so it can be used to transport game data. Once that is finished, then a common communication
interface is defined and the "client connection" is moved to another stage, which is game initialization.

Crates worth to confider for network-level functionalities:

* [tungstenite](https://crates.io/crates/tungstenite) - Lightweight stream-based WebSocket implementation for Rust.
* [tokio-tungstenite](https://crates.io/crates/tokio-tungstenite) - Asynchronous WebSockets for Tokio stack.

The package exchange between client and server will use JSON. The Serde create has an extension for JSON support:
[Serde JSON](https://github.com/serde-rs/json).

## Game-level

The server will need to ensure data consistency. The client's requests, before they have any effect on the game's state,
will need to be validated. This more or less narrows to:

* limiting player's movement against map boundaries,
* limiting player's actions based on their resource usage,
* rejecting client's network packages if received in an invalid state (e.g. corresponding to player's death).

The server will also need to generate responses to certain in-game events, such as:

* combining two beams into one and calculating its angle,
* removing a player from the game if killed,
* changing player's resource levels (such as: health or energy).

To resolve most of those problems, the game needs to implement a general physics-engine. Crates.io offers some
solutions:

* [Parry](https://parry.rs/) - 2D and 3D collision-detection library for the Rust programming language.
* [collision](https://crates.io/crates/collision) - provides collision detection primitives, bounding volumes and
collision detection algorithms.
