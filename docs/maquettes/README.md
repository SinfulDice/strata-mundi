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

# Maquettes — thème de l'interface (fin de v0.1)

> **Choix : thème 1, Pierre** (29/10/2026, voir `docs/decisions.md`, D035). Les deux autres restent ici comme exemples.

Chaque planche montre l'écran titre, l'écran d'options et l'inventaire créatif, capturés dans le jeu. Seules les couleurs de l'interface changent (`assets/ui/theme.ron`) ; la police pixel et la disposition sont identiques.

| Planche | Thème | Idée |
|---|---|---|
| [`theme-1-pierre.png`](theme-1-pierre.png) | **Pierre** (actuel) | Gris neutre, accent doré : discret, laisse la place au monde. |
| [`theme-2-bois.png`](theme-2-bois.png) | **Bois chaleureux** | Panneaux de bois brun, accent doré : rappelle l'établi et la cabane. |
| [`theme-3-parchemin.png`](theme-3-parchemin.png) | **Parchemin** | Panneaux clairs et encre sombre, accent rouille : le carnet d'un explorateur. |

Réponds avec le numéro ou le nom, ou panache (« le bois, mais l'accent du parchemin »). Les couleurs de chaque maquette sont dans `docs/maquettes/themes/<n>-<nom>.ron` ; pour essayer en jeu, copie le fichier sur `assets/ui/theme.ron` (ou appuie sur F5 en jeu après la copie).
