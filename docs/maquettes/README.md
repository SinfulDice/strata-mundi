# Maquettes — choix du style des textures (v0.1, jalon 2)

> **Choix : style 1, classique** (23/10/2026, voir `docs/decisions.md`, D017). Les autres palettes restent ici comme exemples de packs de ressources.

Trois planches d'essai, générées par le jeu (`strata --texture-sheet`) sur le même échantillon. Seule la **palette globale** change : les dessins (motifs procéduraux et sprites) sont identiques.

| Planche | Style | Idée |
|---|---|---|
| [`textures-1-classique.png`](textures-1-classique.png) | **Classique** | Proche de Minecraft : tons naturels, contrastes moyens. |
| [`textures-2-chaleureux.png`](textures-2-chaleureux.png) | **Chaleureux** | Plus saturé, lumière dorée, ombres tirant vers le pourpre. |
| [`textures-3-pastel.png`](textures-3-pastel.png) | **Pastel** | Couleurs douces et claires, ambiance de conte. |

**Contenu de chaque planche** (de gauche à droite, de haut en bas) : textures des blocs (socle, dessus et côté du cactus, minerais de charbon et de diamant, terre, verre, minerai d'or, dessus et côté du bloc d'herbe, gravier, minerai de fer, feuilles, dessus et côté de la bûche de chêne, planches, sable, neige, pierre, eau), puis l'échantillon d'objets et de mob : pioches en diamant, fer, pierre et bois (un seul dessin recoloré), épée en fer, tête de mouton. En dessous, un petit diorama rendu par le moteur.

À noter : l'eau est encore dessinée opaque (la transparence arrive au jalon 3) ; la couleur du ciel est provisoire (v0.2).

## Comment choisir

Réponds simplement avec le numéro ou le nom du style. Tu peux aussi panacher (« le classique, mais l'herbe du chaleureux ») ou demander des retouches (contraste, saturation, une matière en particulier, le dessin des outils…).

Les palettes sont dans `assets/packs/strata/palette.ron` (classique) et `docs/maquettes/styles/<style>/palette.ron` ; pour comparer en jeu :

```sh
cargo run --release -- --resource-packs-dir docs/maquettes/styles --resource-pack pastel
```
