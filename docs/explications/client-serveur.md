# Client et serveur

## À quoi ça sert

Même en solo, Strata Mundi est découpé comme un jeu en ligne : un **serveur** qui décide de tout ce qui arrive dans le monde, et un **client** qui affiche et transmet les envies du joueur. Ainsi, le multijoueur (v2.0) n'aura qu'à remplacer le « tuyau » entre les deux par une connexion réseau, sans réécrire le jeu.

## Comment ça marche

- Le client **n'écrit jamais dans le monde**. Quand le joueur voudra casser un bloc (jalon 6), le client enverra une *intention* (« je veux casser le bloc en x, y, z ») ; le serveur vérifiera que c'est possible, modifiera le monde et préviendra tous les clients.
- Tout échange est un **message** défini dans la crate `protocol` : `ClientMessage` (client → serveur) et `ServerMessage` (serveur → client). Chaque message est **sérialisable** (transformable en octets), donc prêt à passer par le réseau.
- En solo, les messages passent par des **canaux en mémoire** : `local_pair()` crée les deux bouts d'un tuyau, un pour le client (`ClientConnection`), un pour le serveur (`ServerConnection`).
- Si un des deux côtés disparaît, l'autre le voit (`Disconnected`) et s'arrête proprement.

Messages actuels :

| Message | Sens | Rôle |
|---|---|---|
| `ClientMessage::Disconnect` | client → serveur | Le joueur quitte : le serveur s'arrête. |
| `ServerMessage::Tick { tick }` | serveur → client | Numéro du tick qui vient d'être simulé. |

## Où c'est dans le code

- `crates/protocol/src/messages.rs` : les messages.
- `crates/protocol/src/local.rs` : le transport en mémoire.
- `crates/server/src/integrated.rs` : le serveur qui lit les messages du client à chaque tour.
- `crates/client/src/frame.rs` (`receive_messages`) : le client qui lit les messages du serveur.

<details><summary>Côté Rust</summary>

- **`enum`** : un type qui vaut *une* variante parmi plusieurs, chacune pouvant porter des données (`Tick { tick: u64 }`). Le `match` oblige à traiter toutes les variantes : ajouter un message fait échouer la compilation partout où il n'est pas encore géré.
- **`#[derive(Serialize, Deserialize)]`** (crate `serde`) : génère automatiquement le code qui transforme le type en octets et inversement.
- **Canaux `mpsc`** : `std::sync::mpsc::channel()` donne un émetteur (`Sender`) et un récepteur (`Receiver`) utilisables depuis deux threads différents. `try_recv()` lit un message s'il y en a un, sans attendre.

</details>
