# Client et serveur

## À quoi ça sert

Même en solo, Strata Mundi est découpé comme un jeu en ligne : un **serveur** qui décide de tout ce qui arrive dans le monde, et un **client** qui affiche et transmet les envies du joueur. Ainsi, le multijoueur (v2.0) n'aura qu'à remplacer le « tuyau » entre les deux par une connexion réseau, sans réécrire le jeu.

## Comment ça marche

- Le client **n'écrit jamais dans le monde**. Quand le joueur casse un bloc, le client envoie une *intention* (« je veux casser le bloc en x, y, z ») ; le serveur vérifie que c'est possible (portée, place libre…), modifie le monde et prévient tous les clients avec `BlockChanged`.
- Tout échange est un **message** défini dans la crate `protocol` : `ClientMessage` (client → serveur) et `ServerMessage` (serveur → client). Chaque message est **sérialisable** (transformable en octets), donc prêt à passer par le réseau.
- En solo, les messages passent par des **canaux en mémoire** : `local_pair()` crée les deux bouts d'un tuyau, un pour le client (`ClientConnection`), un pour le serveur (`ServerConnection`).
- Si un des deux côtés disparaît, l'autre le voit (`Disconnected`) et s'arrête proprement.

Messages actuels :

| Message | Sens | Rôle |
|---|---|---|
| `ClientMessage::Input` | client → serveur | Les touches de déplacement d'un tick, numérotées (voir [Joueur et interaction](joueur-et-interaction.md)). |
| `ClientMessage::MovementPreferences` | client → serveur | Options de déplacement : saut automatique, montée des demi-blocs. |
| `ClientMessage::Teleport` | client → serveur | Placement initial (`--pos`) ; plus tard `/tp`. |
| `ClientMessage::BreakBlock` / `PlaceBlock` | client → serveur | Intention de casser ou de poser un bloc. |
| `ClientMessage::ViewDistance` | client → serveur | Distance de rendu souhaitée. |
| `ClientMessage::Disconnect` | client → serveur | Le joueur quitte : le serveur s'arrête. |
| `ServerMessage::Welcome` | serveur → client | Dimension, corps du joueur et règles de déplacement. |
| `ServerMessage::PlayerState` | serveur → client | Position officielle du joueur après l'entrée n° `sequence`. |
| `ServerMessage::BlockChanged` | serveur → client | Un bloc a changé. |
| `ServerMessage::Tick { tick }` | serveur → client | Numéro du tick qui vient d'être simulé. |
| `ServerMessage::Column` / `UnloadColumn` | serveur → client | Une colonne de blocs arrive ou doit être oubliée (voir [Monde infini](monde-infini.md)). |

## Où c'est dans le code

- `crates/protocol/src/messages.rs` : les messages.
- `crates/protocol/src/local.rs` : le transport en mémoire.
- `crates/server/src/integrated.rs` : le serveur qui lit les messages du client à chaque tour.
- `crates/client/src/session.rs` (`receive_messages`) : le client qui lit les messages du serveur ; `gameplay.rs` les applique.

<details><summary>Côté Rust</summary>

- **`enum`** : un type qui vaut *une* variante parmi plusieurs, chacune pouvant porter des données (`Tick { tick: u64 }`). Le `match` oblige à traiter toutes les variantes : ajouter un message fait échouer la compilation partout où il n'est pas encore géré.
- **`#[derive(Serialize, Deserialize)]`** (crate `serde`) : génère automatiquement le code qui transforme le type en octets et inversement.
- **Canaux `mpsc`** : `std::sync::mpsc::channel()` donne un émetteur (`Sender`) et un récepteur (`Receiver`) utilisables depuis deux threads différents. `try_recv()` lit un message s'il y en a un, sans attendre.

</details>
