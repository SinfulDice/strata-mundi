# Console, débogage et cinématiques

## À quoi ça sert

Des outils pour tester le monde (commandes), comprendre ce qui se passe (F3), modifier les ressources sans relancer (F5), signaler un plantage, savoir qu'une nouvelle version existe, et faire des plans de caméra scriptés (utilisés plus tard pour les boss et les découvertes).

## Comment ça marche

### La console (T ou /)

Une ligne de saisie en bas de l'écran, les réponses au-dessus :

| Commande | Effet | Exécutée par |
|---|---|---|
| `/help [commande]` | liste les commandes ou en détaille une | client |
| `/tp <x> <y> <z>` | téléporte ; `~` = sa propre coordonnée, `~5` = 5 blocs plus loin | serveur |
| `/give <objet>` | met l'objet dans la case choisie (`stone` = `strata:stone`) | serveur |
| `/gamemode <creative\|survival>` | mode de jeu (survie en v0.3) | serveur |
| `/time <set\|add\|query> [valeur]` | heure du monde : `day`, `noon`, `night`, `midnight` ou un nombre de ticks (une journée = 24 000 ticks = 20 minutes) | serveur |
| `/seed` | graine du monde | serveur |
| `/cinematic <id>` | joue une cinématique des packs | client |

- **Le serveur décide** : la ligne part telle quelle dans un message `ClientMessage::Command` ; le serveur l'analyse, agit et répond par `ServerMessage::Feedback`, qui ne contient **pas de texte** mais une **clé de traduction** et ses arguments (`command-tp-done`, `x = 12.5`…). Chaque joueur lit donc la réponse dans sa langue — utile pour le multijoueur.
- **Historique** : haut/bas reprend les lignes précédentes.
- **Tab** complète le mot en cours : noms de commandes, objets, cinématiques, choix (`set`, `noon`…). Plusieurs possibilités : elles s'affichent au-dessus de la ligne.
- La liste des commandes et de leurs arguments (`protocol::commands::COMMANDS`) est partagée par le client (aide, complétion) et le serveur (exécution).

### F3 : informations de débogage

Images par seconde (moyenne sur une demi-seconde), position, section, direction du regard, colonnes chargées, maillages en attente, bloc visé et son identifiant, **mémoire utilisée** par le jeu (relevée une fois par seconde) et carte graphique. Sur la manette : Select.

### F5 : recharger les ressources

Textures, traductions, écrans d'interface, polices et shaders sont relus sans relancer le jeu : on modifie un pack, on appuie sur F5, on voit le résultat. En développement (`cargo run`), les shaders sont relus depuis `crates/client/src/render/shaders/`. Si un shader ne compile pas, les ressources précédentes restent en place et un message le signale. Les **blocs et objets** ne peuvent pas changer ainsi (le serveur et le monde ouvert s'en servent) : il faut relancer.

### Rapports de plantage

Si le jeu plante (une « panique » en Rust), un crochet écrit `crash-reports/crash-<date>.txt` : version, système, message, pile d'appels et les 60 dernières lignes du journal (gardées en mémoire en permanence). Au lancement suivant, l'écran titre propose **« Signaler ce plantage »** : le navigateur s'ouvre sur une issue GitHub pré-remplie avec le rapport. Rien n'est jamais envoyé sans ce clic.

### Vérification des mises à jour

Au lancement, un fil séparé demande à GitHub la dernière version publiée (une seule requête, 8 secondes au plus). Si elle est plus récente, l'écran titre l'annonce. Désactivable dans Options → Jeu ; hors ligne, rien ne se passe.

### Cinématiques

Un plan de caméra décrit dans un pack, `cinematics/<nom>.ron` :

```ron
(
    keyframes: [
        (time: 0.0, pos: (0.0, 2.0, 0.0), yaw: 0.0, pitch: 0.0),
        (time: 3.0, pos: (-12.0, 10.0, -12.0), yaw: 135.0, pitch: -25.0, subtitle: Some("cle-de-traduction")),
        (time: 6.0, pos: (12.0, 18.0, -12.0), yaw: 225.0, pitch: -35.0),
    ],
    skippable: true,   // Échap / B pour passer (par défaut)
    letterbox: true,   // bandes noires (par défaut)
)
```

- Les positions sont **relatives** au point d'où le plan est lancé : le même plan sert partout.
- La trajectoire passe par chaque point clé en douceur (**spline de Catmull-Rom**) ; les angles tournent par le plus court chemin (de 350° à 10°, on passe par 360°, pas par 180°).
- Pendant le plan, le joueur ne bouge plus et le HUD est remplacé par les bandes et le sous-titre ; à la fin, la vue revient aux yeux du joueur.

## Où c'est dans le code

- `crates/protocol/src/commands.rs` : liste des commandes, `Feedback`.
- `crates/server/src/commands.rs` : exécution côté serveur.
- `crates/client/src/screens/console.rs` : la console (saisie, historique, complétion).
- `crates/client/src/debug_overlay.rs` : F3. `crates/client/src/session/reload.rs` : F5.
- `crates/app/src/crash.rs`, `crates/app/src/updates.rs` : plantages et mises à jour.
- `crates/core/src/cinematic.rs` (format, trajectoire) et `crates/client/src/cinematic.rs` (lecture).

<details><summary>Côté Rust</summary>

- **`std::panic::set_hook`** : remplace ce que fait Rust quand le programme panique ; on écrit le rapport, puis on rappelle le comportement d'origine.
- **`try_lock`** dans le crochet de panique : si la panique arrive pendant qu'un autre fil écrit dans le journal, attendre le verrou bloquerait le jeu pour toujours ; on préfère un rapport sans journal.
- **Portées d'erreur wgpu** (`push_error_scope`) : les erreurs de validation du GPU (un shader faux) sont récupérées au lieu de faire planter le jeu.

</details>
