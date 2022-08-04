# System overview

Spectrum is a real-time multiplayer browser game.

## Characteristics

* The game takes place on a finite plane with limited number of players.
* In a match there are *two* teams.
* Each player controls one drone.
* Each drone can laser a beam of a specific color, the color determines the beam's effect.
* Beams can be combined to create a new beam of a different color.

### Technical characteristics

* The game is developed in a client-server architecture.
* Client (the browser) initializes the connection with the server via a
    [WebSocket](https://www.rfc-editor.org/rfc/rfc6455.html).
* Network packages have well-defined structure: metadata + payload (if any).
* The server holds the state of the game.
* Players can send requests to modify the state. The server can reject such requests if their validation fails.
* Once the state has been modified, it's propagated to other players.
