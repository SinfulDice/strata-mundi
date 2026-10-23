# Guide des textures et des packs de ressources

Toutes les images du jeu peuvent être remplacées, sans toucher au code, par un **pack de ressources**.

## Comment le jeu fabrique ses textures

Chaque texture a un **identifiant** égal à celui du bloc ou de l'objet qui l'utilise, par exemple `strata:stone`. Le jeu la cherche dans les packs, du moins prioritaire au plus prioritaire ; la dernière trouvée gagne :

1. le contenu du jeu : `assets/packs/<namespace>/textures/` ;
2. les packs de ressources activés : `resourcepacks/<nom>/textures/`.

Une texture peut être :
- une **image PNG** `stone.png` de **16×16, 32×32 ou 64×64** pixels (carrée) ;
- une **recette** `stone.ron`, dessinée par le jeu à partir de la **palette globale** (`palette.ron`).

Si un même dossier contient les deux, le PNG gagne. Les tailles peuvent être mélangées : le jeu agrandit toutes les textures à la plus grande taille présente (pixels nets, sans flou).

## Arborescence d'un pack de ressources

```
resourcepacks/
  mon-pack/
    pack.ron                  # (namespace: "mon_pack", description: "Mes textures")
    palette.ron               # facultatif : recolore tout le jeu
    textures/
      strata/                 # remplace des textures du jeu de base (namespace strata)
        stone.png
        oak_planks.png
```

- `namespace` : nom court en minuscules, chiffres et `_`.
- Les fichiers posés directement dans `textures/` appartiennent au namespace du pack ; pour **remplacer** une texture du jeu, place-la dans `textures/strata/`.
- Noms de fichiers : l'identifiant sans le namespace (`strata:oak_planks` → `oak_planks.png`), en minuscules.

## Pas à pas : remplacer la texture de la pierre

1. Crée le dossier `resourcepacks/ma-pierre/` à côté du jeu.
2. Crée `resourcepacks/ma-pierre/pack.ron` :
   ```ron
   (namespace: "ma_pierre", description: "Ma pierre à moi")
   ```
3. Dessine une image carrée de 16×16 (ou 32×32, 64×64) pixels dans ton logiciel préféré (Aseprite, LibreSprite, GIMP, Piskel…) et enregistre-la en PNG sous `resourcepacks/ma-pierre/textures/strata/stone.png`.
4. Lance le jeu avec le pack activé :
   ```sh
   strata --resource-pack ma-pierre
   ```
   (le menu d'activation et d'ordre des packs arrive avec les menus du jeu, jalon 7).
5. Pour vérifier d'un coup d'œil : `strata --resource-pack ma-pierre --texture-sheet planche.png` produit une planche de toutes les textures.

Si l'image a une mauvaise taille ou n'est pas un PNG valide, le jeu l'indique dans ses messages (fichier et raison) et affiche à la place un damier magenta et noir.

## Recolorer tout le jeu avec une palette

Toutes les textures dessinées par le jeu prennent leurs couleurs dans des **rampes** nommées (du plus sombre au plus clair) de `palette.ron`. Un pack de ressources peut en remplacer quelques-unes :

```ron
// resourcepacks/automne/palette.ron
{
    "grass": ["#6b5a1e", "#8a7426", "#a88e30", "#c4a83c"],
    "oak_leaves": ["#7a3a12", "#9a4a18", "#b85e20", "#d4782c"],
}
```

Les rampes existantes sont listées dans `assets/packs/strata/palette.ron`. Couleurs en `#rrggbb` ou `#rrggbbaa` (transparence).

## Recettes de textures (pour aller plus loin)

Un fichier `textures/<nom>.ron` décrit comment dessiner la texture :

| Recette | Effet | Exemple |
|---|---|---|
| `Material(ramp, pattern)` | Une matière entière | `Material(ramp: "stone", pattern: Noise(cells: (4, 4), octaves: 3))` |
| `Overlay(base, ramp, pattern)` | Un motif peint sur une autre texture | côté du bloc d'herbe : `Overlay(base: "strata:dirt", ramp: "grass", pattern: TopFringe(depth: 4))` |
| `Ore(base, ramp, clusters)` | Petits amas de minerai éclairés en haut à gauche | `Ore(base: "strata:stone", ramp: "gold", clusters: 4)` |
| `Sprite(colors, rows)` | Pixel art écrit en texte : 16 lignes de 16 caractères, `.` = transparent | voir `textures/wooden_pickaxe.ron` |
| `Recolor(base, swap)` | Le même dessin avec d'autres rampes | `Recolor(base: "strata:wooden_pickaxe", swap: {"oak_wood": "iron"})` |

Motifs (`pattern`) : `Noise(cells: (x, y), octaves)` (grain ; `(8, 2)` donne des stries verticales), `Rings(border)` (cernes d'une bûche), `Planks(boards)` (planches), `Leaves(holes)` (feuillage ajouré), `Stripes(period)` (côtes du cactus), `TopFringe(depth)` (frange en haut), `Frame` (cadre de verre).

Règles de style : lumière unique venant d'**en haut à gauche** ; contour des objets et des mobs dans un ton **plus foncé de leur propre couleur** ; couleurs prises dans la palette globale pour garder un ensemble harmonieux.

## Formats à venir

Textures animées (bande verticale + fichier de timing), textures connectées (verre, bibliothèques) et variantes aléatoires par bloc seront documentées ici quand elles arriveront.
