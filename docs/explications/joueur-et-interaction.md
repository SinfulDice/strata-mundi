# Joueur et interaction

## À quoi ça sert

Le joueur marche, saute, sprinte, s'accroupit, vole en créatif, vise un bloc, le casse, en pose un autre. Tout cela doit être **immédiat** à l'écran, mais c'est quand même le **serveur** qui décide (règle d'or du projet, et condition pour le multijoueur).

## Comment ça marche

### Une seule physique, des deux côtés

La physique du joueur (`step`) est écrite **une fois**, dans `core`, et tourne à la fois sur le client et sur le serveur, au même rythme fixe de 20 pas par seconde. Avec les mêmes entrées, elle donne exactement le même résultat des deux côtés.

- **Boîte de collision** de 0,6 × 1,8 bloc (yeux à 1,62), comme Minecraft.
- **Gravité**, saut d'environ 1,25 bloc, marche 4,3 blocs/s, sprint 5,6, accroupi 1,3.
- **Collisions axe par axe** et par petits pas (jamais plus de 0,4 bloc d'un coup) : même très vite, on ne traverse pas un mur.
- **Accroupi** : on ne tombe pas du bord d'un bloc.
- **Montée automatique d'un demi-bloc** (dalles, escaliers à venir) et **saut automatique** quand on marche contre une marche d'**un** bloc (jamais contre un mur, jamais accroupi) : les deux se règlent (`ControlSettings`).
- **Vol créatif** : deux appuis rapides sur saut (moins de 0,3 s) ; Espace monte, Maj descend ; se poser coupe le vol.
- **Nage** : dans un liquide, la même fonction partagée ralentit, fait couler doucement et pousse hors de l'eau contre une berge (voir [Fluides](fluides.md)).
- **Assis dans un bateau** : le joueur ne prédit plus son corps ; ses entrées pilotent le bateau côté serveur (voir [Bateaux et entités](bateaux-et-entites.md)).
- Les colonnes pas encore chargées comptent comme pleines : personne ne tombe dans un monde qui n'est pas arrivé.

### Prédiction et réconciliation

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Serveur
    C->>C: entrée n°41 : avance tout de suite (prédiction)
    C->>S: Input { sequence: 41, … }
    C->>C: entrée n°42 : avance encore
    C->>S: Input { sequence: 42, … }
    S->>S: applique 41 avec la même physique
    S-->>C: PlayerState { sequence: 41, corps officiel }
    C->>C: repart du corps officiel et rejoue 42
```

1. À chaque pas, le client lit les touches, fabrique une `MoveInput` (direction voulue, regard, saut, sprint…), **l'applique tout de suite** à son joueur et l'envoie numérotée au serveur.
2. Le serveur range ces entrées et les applique **au rythme de son horloge** : une par tick en moyenne, avec une petite réserve de 3 pour rattraper un retard (un client ne peut donc pas aller plus vite en envoyant plus de messages). Un client qui n'envoie plus rien pendant une demi-seconde ne flotte pas : son joueur retombe. Le serveur renvoie le corps officiel et le numéro de la dernière entrée appliquée.
3. Le client **repart du corps officiel** et rejoue les entrées que le serveur n'a pas encore vues. Si tout va bien, on retombe exactement au même endroit et rien ne bouge à l'écran ; si le serveur a corrigé (téléportation, triche), le client suit le serveur.
4. L'affichage **interpole** entre les deux derniers pas : le mouvement est fluide à 30, 60 ou 144 images par seconde alors que la physique ne tourne qu'à 20.

### Viser, casser, poser

- **Raycast** (algorithme d'Amanatides et Woo) : depuis les yeux, on avance bloc par bloc dans la direction du regard jusqu'à 5 blocs, en sachant par quelle face on est entré. Le bloc visé est entouré d'un **contour**.
- **Casser** (clic gauche) : instantané en créatif. Le client envoie `Action { id, Break }` ; le serveur vérifie la position (dans la bordure et la hauteur du monde), la portée (6 blocs, un peu plus que le client pour les arrondis) et la dureté (hors créatif, le socle est incassable), puis renvoie `BlockChanged`.
- **Poser** (clic droit) : `Action { id, Place { contre, face, bloc } }`. Le serveur vérifie : bloc posable, support solide, case libre, dans la hauteur du monde, à portée, et pas dans le corps d'un joueur.
- **Rythme sans clic perdu** : le serveur fait au plus une action toutes les 2 ticks (10 par seconde). Un clic arrivé trop tôt n'est plus jeté : il attend dans une **file** (8 clics au plus ; au-delà il est refusé, mais à son tour) et passe dès que son tour vient, avec toutes les vérifications faites à ce moment-là. Chaque clic reçoit une réponse `ActionDone(id)`, dans l'ordre des clics ; un clic refusé renvoie d'abord l'état réel des cases visées que le joueur voit. Les clics sur un bateau (monter, retirer) passent par la même file. Le client ne prédit pas plus de 8 clics sans réponse.
- **Prédiction des blocs** : le client n'attend pas le serveur pour **afficher** le résultat d'un clic. Il calcule avec les mêmes règles (case visée, variante orientée, support des plantes, pas dans le joueur) ce que le serveur devrait faire, l'affiche tout de suite (remaillage, lumière provisoire) et joue le son et les particules **une seule fois**. Tant que le clic n'a pas sa réponse, les `BlockChanged` du serveur pour cette case sont mis de côté ; à la réponse, la case prend le **dernier état dit par le serveur**, sans son ni particule (un refus défait la prédiction en silence, et une prédiction écrasée par un changement plus récent ne revient jamais). Une colonne rechargée oublie ses prédictions. Seau, briquet et bateau ne sont pas prédits.
- **Orientation** : les bûches se posent **couchées dans l'axe de la face cliquée**. Dans le pack, il suffit d'écrire `placement: Axis` : au chargement, le jeu crée tout seul les variantes couchées `…__x` et `…__z`, sans objet ni nom propre (elles s'appellent comme la bûche debout). Le maillage tourne la texture d'écorce pour que les fibres suivent la bûche.
- **Seau et bateau** : clic droit avec l'objet en main envoie `Action { id, Use }` (même file que casser et poser) ; le serveur pose ou retire une source, ou crée un bateau. Clic droit sur un bateau : y monter (la visée teste aussi les boîtes des entités).
- **Maintenir le bouton** répète l'action toutes les 0,25 s.
- **Prise de bloc** (clic molette) : met l'objet du bloc visé dans la barre.
- **Nom du bloc visé** en haut de l'écran (façon mod Jade), désactivable (`show_block_name`). L'outil requis s'y ajoutera avec la survie (v0.3).

### Barre et inventaire créatif

- **Barre** de 9 cases : touches 1 à 9, molette, L/R à la manette.
- **Inventaire créatif** (E ou X) : onglets par catégorie (Tout, Construction, Nature… venant des packs), champ de **recherche** (sur le nom traduit), grille de tous les objets. Un clic, A ou une touche 1-9 met l'objet dans la barre. Voir [Interface](interface.md).

## Où c'est dans le code

| Quoi | Où |
|---|---|
| Physique partagée, boîte de collision | `crates/core/src/physics/` (`Body`, `MoveInput`, `step`, `Aabb`) |
| Raycast | `crates/core/src/raycast.rs` |
| Joueur côté serveur (entité, validations) | `crates/server/src/game.rs`, `systems.rs`, `rules.rs` |
| Prédiction, réconciliation, double saut | `crates/client/src/player.rs` (`LocalPlayer`) |
| Touches → `MoveInput`, caméra | `crates/client/src/gameplay.rs`, `look.rs` |
| Visée, casser, poser, prise de bloc | `crates/client/src/interaction.rs` |
| File des clics, réponses `ActionDone` | `crates/server/src/game/actions.rs` |
| Prédiction des blocs | `crates/client/src/prediction.rs`, `streaming.rs` (`predict`, `receive_block`, `acknowledge`) |
| Contour du bloc visé | `crates/client/src/render/outline.rs` |
| Variantes orientées | `crates/core/src/content/` (`Placement`, `Orientation`, `insert_turned`) |
| Barre | `crates/client/src/hotbar.rs` |

<details><summary>Côté Rust</summary>

- **`impl Fn(i32, i32, i32) -> bool`** (et le trait `Terrain` qui y ajoute la lecture des liquides et la glissance) : la physique ne sait rien des chunks ; on lui passe une *fonction* qui répond « ce bloc est-il solide ? ». Le serveur lui donne ses colonnes, le client les siennes, les tests un simple sol plat. C'est ce qui permet d'écrire la physique une seule fois.
- **`VecDeque`** : une file où l'on ajoute à la fin et retire au début en temps constant, idéale pour les entrées en attente.
- **`wrapping_add` / `wrapping_sub`** : les numéros d'entrée sont des `u32` qui repartent à zéro après 4 milliards ; ces opérations le font sans plantage, et la comparaison `(a - b) as i32 <= 0` reste juste au passage.
- **`f64` pour les positions** : à 10 millions de blocs de l'origine, un `f32` n'a plus que ~1 bloc de précision ; un `f64` reste précis au micromètre.

</details>
