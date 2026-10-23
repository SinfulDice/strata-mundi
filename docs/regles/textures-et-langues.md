# Textures, packs de ressources et langues

## Textures et packs de ressources

**Méthode de production (hybride)**
- **Matériaux procéduraux** (pierre, terre, sable, bois, feuilles, minerais, laine, métaux…) : palette de 4 à 8 couleurs + recette de génération (bruit, fissures, cernes et fibres, amas de minerai, tramage des feuilles), déterministe par seed.
- **Objets, outils, armes, interface et textures de modèles** : **sprites en pixel art texte** dans les packs RON (grilles de caractères, une lettre = une couleur de palette). Un même dessin se décline par matériau en changeant la palette.
- **Règles de style communes** : lumière unique venant d'en haut à gauche, **contour plus foncé de la couleur de l'objet** sur objets et mobs, palette globale harmonisée.
- **Résolution des modèles** : 16 px par bloc pour les mobs et le joueur, **32 px par bloc pour les boss et les dieux**.
- **Planche de contact** : `app --texture-sheet <png>` regroupe toutes les textures sur une image, inspectée par `visual-tester` et jointe au rapport de chaque version.
- **Direction artistique à valider** : au jalon 2 de la v0.1, produire **2 à 3 planches d'essai** avec des palettes différentes (ex. proche de Minecraft, plus saturée et chaleureuse, pastel) sur un même échantillon (blocs de base, outils, un mob), puis **s'arrêter** pour que l'utilisateur choisisse. Consigner le choix dans `docs/decisions.md`.


- Style **inspiré de Minecraft et des mods cités** (Create, Sophisticated Backpacks, Waystones, Twilight Forest, Aether, MobHeads) pour la lisibilité et l'ambiance, mais **textures originales** générées par code : ne recopie jamais une texture existante.
- **Remplacement simple et documenté** : un **pack de ressources** est un dossier `resourcepacks/<nom>/` (textures, sons, polices) avec un `pack.ron` ; menu d'activation et d'ordre de priorité dans le jeu.
- Résolutions acceptées : **16, 32 et 64 px** (mélangeables, mise à l'échelle automatique).
- Formats documentés : textures animées (bande verticale + fichier de timing), **textures connectées** (verre, bibliothèques…), **variantes aléatoires** par bloc.
- Chaque texture a un identifiant stable égal à celui de son bloc ou objet ; la liste complète est générée dans le wiki.
- **`docs/guide-textures.md`** (et sa page wiki) : arborescence, nommage, formats, exemples, pas à pas « remplacer la texture de la pierre ».

## Langues

- **Français**, **anglais** et **LOLcat** (`lol`, langue humoristique façon « I CAN HAZ CHEEZBURGER »), pour le **jeu et le wiki**.
- Le LOLcat est une langue complète, pas un filtre automatique : chaque texte a sa traduction rédigée à partir de l'anglais. Règles : orthographe phonétique, pluriels en -Z, « teh », grammaire de bébé (« I CAN HAS »), majuscules, références aux chats (kitteh, cheezburger), lisible avant tout.
- **Glossaire** dans `assets/packs/strata/lang/lol-glossary.ron (les traductions elles-mêmes sont en Fluent `.ftl`)` : un terme anglais = une seule traduction LOLcat, réutilisée partout (ex. Crafting Table → CRAFTIN TABUL, Diamond Pickaxe → SHINY DIGGY STIK, You died! → U DIED. OH NOES!!1). Le compléter à chaque nouveau terme.
- Le mode strict de `wikigen` et un test de traduction échouent si une clé manque dans l'une des trois langues.
