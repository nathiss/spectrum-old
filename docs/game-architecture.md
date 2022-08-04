# Game architecture

Ideally the server will be easily-scalable. Each server instance will be able to host many games. Each game will
aggregate a relatively small amount of players and manage interactions between them. The only requirement in this case
is that all players from the same game need to be connected to the same server instance.

![Single Instance Server Overview](./asserts/SingleRealm.png)

There's no need to preserve a game state that will spawn across all server instances. For any such state a consensus
algorithms could be used to select a master node (which will hold the state). An implementation of such algorithm is
[The Raft Consensus Algorithm](https://raft.github.io/).
