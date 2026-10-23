# Interface (HUD et menus)

## À quoi ça sert

Tout ce qui se dessine **par-dessus** le monde : le viseur, la barre d'objets, le nom du bloc visé, l'inventaire créatif et les menus (voir [Menus et écran titre](menus.md)). L'interface est en pixel art, faite maison, et se pilote aussi bien à la souris qu'au clavier ou à la manette.

## Comment ça marche

### Des écrans décrits en fichiers

Chaque écran est un fichier RON dans `assets/ui/` qui dit **quoi est où** ; le code Rust dit seulement **comment ça réagit**. Exemple, le HUD :

```ron
Screen(
    root: [
        Crosshair(id: "crosshair", anchor: Center, at: (0, 0)),
        Label(id: "target", anchor: Top, at: (0, 4), text: "@target", boxed: true),
        Slots(id: "hotbar", anchor: Bottom, at: (0, -3), count: 9),
    ],
)
```

- **Ancre** (`anchor`) : le point du parent auquel l'élément s'accroche (centre, haut, coin bas-gauche…) ; **`at`** : un décalage en pixels d'interface. C'est ce qui permettra de déplacer chaque élément dans l'éditeur de HUD (jalon 7).
- **Éléments disponibles** : `Shade` (assombrit le fond), `Panel` (fenêtre avec enfants), `Label` (avec `size` pour les titres, comme le logo), `Button`, `TextField`, `Slider` (curseur), `List` (lignes avec image, comme la liste des mondes), `KeyGrid` (clavier à l'écran), `Tabs`, `ItemGrid`, `Slots`, `Crosshair`.
- **Texte riche** : dans les traductions, `<key:Inventory>` affiche la touche associée dans une petite case, `<c:accent>…</c>` colore le texte.
- **Textes** : une clé de traduction (`ui-search`) ou `@nom` pour un texte fourni par l'écran (le nom du bloc visé). Un test vérifie que chaque clé existe en fr, en et lol.
- **Thème** : `assets/ui/theme.ron` donne toutes les couleurs. Le thème actuel est provisoire ; des maquettes seront proposées en fin de v0.1.

### Dessin

- **Pixels d'interface** : l'interface est pensée pour une petite résolution (320 × 220 au minimum) puis agrandie d'un facteur entier (×2, ×3, ×4) selon la fenêtre, pour des pixels nets.
- **Un seul atlas** (une seule image sur la carte graphique) contient la police, les icônes de tous les objets et un pixel blanc pour les rectangles de couleur. Tout l'écran se dessine en un appel.
- **Police pixel** décrite dans le pack (`fonts/pixel.ron`) : chaque caractère est un petit dessin en `#` et `.` ; les accents (é, è, ç…) sont dessinés une fois et posés automatiquement sur les lettres. Un pack de ressources peut la remplacer.
- **Icônes des blocs** : calculées au lancement, sur le processeur, en vue isométrique à partir des textures (dessus clair, côtés plus sombres). Elles suivent donc automatiquement les packs de ressources.

### Navigation

- **Souris** : survol = focus, clic = action, molette.
- **Clavier et manette** : les flèches ou la croix déplacent le **focus** vers l'élément le plus proche dans cette direction (en préférant ceux bien alignés) ; Entrée/A valide, Échap/B revient. Le focus est toujours visible (cadre clair).
- **Saisie de texte** : au clavier, on tape directement dans le champ ; à la manette, un clavier à l'écran s'ouvre (voir [Menus](menus.md)).
- **Taille** : automatique (le plus grand facteur entier qui tient) ou choisie dans les options (×1 à ×6).
- Quand un menu est ouvert, le curseur est libéré et le joueur s'arrête, mais **le monde continue** (comme Minecraft).

## Où c'est dans le code

| Quoi | Où |
|---|---|
| Fichiers d'écran, thème | `assets/ui/*.ron` |
| Format des écrans | `crates/client/src/ui/def.rs` (`ScreenDef`, `Node`) |
| Placement, dessin des éléments | `crates/client/src/ui/widgets/`, `rect.rs`, `painter.rs`, `rich.rs` |
| Atlas, icônes | `crates/client/src/ui/atlas.rs`, `crates/core/src/texture/isometric.rs` |
| Police | `crates/core/src/font.rs`, `assets/packs/strata/fonts/pixel.ron` |
| Navigation au focus | `crates/client/src/ui/focus.rs` |
| Comportement des écrans (HUD, inventaire) | `crates/client/src/screens/` |
| Passe de rendu | `crates/client/src/render/ui_pipeline.rs`, `shaders/ui.wgsl` |

<details><summary>Côté Rust</summary>

- **`trait ScreenModel`** : chaque écran fournit les mêmes fonctions (« quels objets dans cette grille ? », « quel élément est sélectionné ? »). Le code de dessin ne connaît que ce trait, pas l'inventaire ni le HUD : ajouter un écran ne touche pas au dessin.
- **`enum Node` avec `#[derive(Deserialize)]`** : `serde` lit directement le RON en un arbre de valeurs Rust ; une faute de frappe dans le fichier donne une erreur avec ligne et colonne.
- **`BTreeMap<char, …>`** pour la police : un dictionnaire trié, donc l'atlas est toujours construit dans le même ordre (images reproductibles).

</details>
