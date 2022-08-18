# Glossary

This file contains the definitions of all special terms used in the project.

| Term | Definition |
| ---- | ---------- |
| Connection | A network-layer abstraction used to communicate with a single network peer. |
| Client | A server-layer abstraction used to communicate with a single network peer. It manager serialization and
deserialization of binary packages. |
| Player | A game-level abstraction representing a single person playing the game. It manages bidirectional
communication with the peer. |
| Drone | An avatar the player controls. It is an ECS entity aggregating components such as: position, velocity,
health. |
| Beam | A laser that can be shoot from a drone. It is an ECS entity aggregating components such as origin point,
direction, beam type. |
