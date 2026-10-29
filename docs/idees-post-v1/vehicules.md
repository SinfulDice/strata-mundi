# Véhicules

**Statut : À valider.** Version proposée : v1.3.

Véhicules en fin de progression mécanique, après le transport mécanique de la v0.8.

## Types envisagés

**Décidé :** tous les véhicules sont utilisables dans toutes les dimensions ; les **véhicules volants** sont mis en avant dans Machina Caeli.

- **Terrestres** : voiture, véhicule tout-terrain, camion de transport avec inventaire.
- **Volants** : dirigeable, planeur motorisé, ornithoptère ; ils prennent tout leur sens dans l'archipel de Machina Caeli.
- **Aquatiques** : bateau à moteur, sous-marin pour l'Abyssus (lien avec le scaphandre mécanique).
- Style **steampunk**, cohérent avec les armes à feu (chaudières, hélices en laiton, rivets).

## Fonctionnement

- Construits en assemblant des pièces (châssis, moteur, roues ou hélices, réservoir, sièges, stockage), sur le modèle des améliorations modulaires.
- **Énergie (décidé)** : selon le véhicule. Les gros véhicules (camion, dirigeable, sous-marin) brûlent un **carburant** à fabriquer (charbon, huile, vapeur) ; les petits (planeur motorisé, petite voiture) utilisent une **énergie stockée** (ressorts, batteries) rechargée par le réseau mécanique de la v0.8.
- **Combat (décidé)** : armes montées (tourelles, canons fixes) **et** tir du passager avec sa propre arme.
- **Risques (décidé)** : les véhicules peuvent être **détruits** par les mobs et les explosions, et **volés** par des PNJ (bandits, pirates). En plus de l'usure normale.
- Durabilité et réparation, comme l'équipement.
- **Personnalisation visuelle (décidé)** : teintures, pièces décoratives, blason.
- Plusieurs places (le multijoueur existe depuis la v1.0).
- Rangement possible : garage, ou véhicule replié en objet.

## Contraintes de design

- **Tout-terrain libre (décidé)** : pas de routes à construire, aucune différence de vitesse selon le terrain.
- **Pas de voyage rapide** en dehors des balises : un véhicule va plus vite qu'un cheval mais reste un déplacement réel dans le monde.
- Voie mécanique uniquement (les montures magiques restent l'équivalent côté magie).
- Tout se règle : véhicules désactivables, dégâts de collision, consommation.

## Technique et performances

- **Point critique** : un véhicule rapide force la génération et le maillage de chunks. Sur le DS (4 cœurs, 2 travailleurs), **ralentissement automatique (décidé)** : le véhicule ralentit seulement quand le monde devant lui n'est pas encore chargé, puis reprend sa vitesse. À valider par des mesures `--bench-scene`.
- Physique simple et déterministe côté serveur (le client envoie des intentions, comme pour le reste).
- Lien possible avec l'idée « structures mobiles et trains » de la roadmap.
