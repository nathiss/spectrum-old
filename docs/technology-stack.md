# Technology stack

The game's server will be written in Rust.

## General

For managing custom settings this project will use [Serde](https://serde.rs/) crate for serialization and
deserialization of data. More specifically the [Serde JSON](https://github.com/dtolnay/serde-json) create for JSON
support.

The server must allow multiple connections from many endpoints and must be able to handle them. The Rust programming
language has a built-in support for asynchronous programming. It does not however provide any runtime, so it must be
provided as a third-party solution. A popular choice is a [Tokio](https://crates.io/crates/tokio) create.

Rust ecosystem provides a [log](https://crates.io/crates/log) create for abstracting logging interface from its
implementation. There are multiple logger implementations to choose from:

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

The server and clients will use [Google's Protobuf](https://developers.google.com/protocol-buffers) serialization
mechanism for exchanging data. To decrease the size of the network packets, the system will serialize Protobuf messages
to the binary format.

## Game-level

One of the widely used patterns in game development is
[Entity Component System](https://en.wikipedia.org/wiki/Entity_component_system) (or ECS for short).

One of the most popular ECS in the Rust ecosystem is [specs](https://crates.io/crates/specs). It allows you to introduce
data-driven approach into your system. Specs official [Book](https://specs.amethyst.rs/docs/tutorials/).

```rust
#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Position {
    x: f32,
    y: f32,
}

let mut world = World::new();
world.register::<Position>();

struct HelloWorld;

impl<'a> System<'a> for HelloWorld { /* ... */ }

let mut hello_world = HelloWorld::new();
hello_world.run_now(&world);
world.maintain();
```

The example above shows a basic example of specs. Calling `.maintain()` on the world makes all entities created or
deleted while the system was running to be preserved on the system.

The library has the following core concepts:

* `Dispatcher` - it manages multiple systems and the relations between them.
* `Resource` - a data shared between multiple systems that can be either read or written to. It can be useful to
calculate the position delta based on the velocity and `Duration`.
* `System Data` - a type related to `System<'a>` which defines which components/resources will be accessed. Accessing
the same component/resource twice within a single `System Data` tuple will result in a panic.
* `Entities<'a>` - a `System Data` which allows the system to access entities and manage them via `.create()` or
`.delete()` method. It is important to call `.maintain()` on the world later to make changes introduced by the system
persistent.

```toml
specs = { version = "*", features = ["shred-derive"] }
```

Enabling this feature allows you to extract a system's `System Data` into a separate struct:

```rust
#[derive(SystemData)]
pub struct MySystemData<'a> {
    positions: ReadStorage<'a, Position>,
    velocities: ReadStorage<'a, Velocity>,
    forces: ReadStorage<'a, Force>,

    delta: Read<'a, DeltaTime>,
    game_state: Write<'a, GameState>,
}
```

When doing `.join()` on many components it's possible to exclude some of them and get an iterator over entities which
do not have the excluded component. It can be done with the `!` operator:

```rust
for (ent, pos, vel, ()) in (
    &*entities,
    &mut pos_storage,
    &vel_storage,
    !&frozen_storage,
).join() {
    println!("Processing entity: {:?}", ent);
    *pos += *vel;
}
```

`FlaggedStorage` can be used to mark components as modified and therefore only those will be process on the next
system tick.

### Collision detection system

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
