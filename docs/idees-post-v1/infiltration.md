# Machina Caeli armée et infiltration

**Statut : À valider.** Version proposée : v1.3.

## Peuples armés

- Les habitants de Machina Caeli (automates conscients, ingénieurs exilés) se défendent avec des **armes à feu** : c'est leur culture militaire, cohérente avec un monde façonné par Ventus et Fulgur.
- **Décidé :** ils utilisent **les mêmes armes que le joueur** (voir `armes-a-feu.md`), qu'on peut leur acheter, leur prendre ou fabriquer.
- Gardes, patrouilles, tourelles et postes de garde dans leurs cités et bases.
- Ils restent un peuple avec sa réputation : hostiles ou neutres selon la réputation du joueur, pas des ennemis par défaut.
- **Relations entre cités (décidé)** : automates et ingénieurs s'allient ou se méfient selon les cités. Réputation par cité, et certaines missions opposent deux cités rivales.

## Système de discrétion

**Décidé :** système possible partout dans le jeu (bastion gobelin, villages fanatiques, structures…), mais nettement plus poussé dans Machina Caeli (tourelles, caméras, alarmes, missions dédiées).

- **Vision** : chaque garde a un cône de vision, une portée qui dépend de la lumière (niveaux de lumière voxel déjà calculés) et de la météo.
- **Bruit** : marcher, courir, sauter, casser un bloc ou tirer produit du bruit ; s'accroupir et certains matériaux de sol l'atténuent.
- **États d'alerte** : calme → suspicieux (enquête) → alerte (poursuite, renforts) → recherche → retour au calme après un délai.
- **Alarmes** : un garde qui repère le joueur peut déclencher une alarme qui alerte la zone.
- **Corps et traces** : un garde neutralisé peut être découvert par une patrouille.
- **Déguisements** : les têtes de mobs qui trompent la détection existent déjà ; à étendre avec des tenues complètes.
- **Équipement de discrétion** : bottes silencieuses, cape d'ombre, armes silencieuses, gadgets mécaniques (leurre sonore, brouilleur de tourelle, grappin). Côté magie : sorts d'invisibilité, de silence, d'illusion.
- **Neutraliser caméras et tourelles (décidé)** : les pirater (mini-jeu de circuit pour les couper ou les retourner), saboter l'énergie de la zone, les détruire (bruyant, déclenche souvent l'alerte), ou utiliser des sorts (illusion, silence, foudre qui court-circuite).
- Indicateur de détection au HUD, désactivable.

## Missions d'infiltration

- Missions façon *Metal Gear* : entrer dans une base, atteindre un objectif (voler des plans, libérer un prisonnier, saboter une machine, récupérer une pièce), ressortir.
- **Plusieurs façons de réussir** : infiltration complète sans être vu, neutralisation discrète, ou assaut direct (plus dangereux). Cohérent avec le principe des voies multiples.
- **Tuer les gardes (décidé)** : toujours possible. Un meurtre **non remarqué** n'a pas de conséquence ; un meurtre **remarqué** (témoin, corps découvert, alarme) fait baisser la réputation et les récompenses.
- **Être repéré (décidé)** : dépend de la mission. Certaines tolèrent l'alerte (on peut finir en combattant ou en fuyant), d'autres échouent immédiatement. Le type de mission est annoncé avant de l'accepter.
- Récompenses bonus si aucune alarme n'est déclenchée.
- Bases conçues comme des structures en packs : patrouilles décrites par des trajets, emplacements de caméras et de tourelles, conduits et passages secondaires.
- **Écrites et générées (décidé)** : quelques grandes missions écrites à la main, plus des contrats générés et répétables (base, patrouilles et objectif tirés selon la seed).
- Origine des missions : PNJ, tableau de quêtes des tavernes, réputation auprès des différents peuples.
- Scénario et textes des missions : `[LORE À ÉCRIRE]`, à rédiger en séance de lore.

## Technique et performances

- L'IA de discrétion s'appuie sur l'IA comportementale de la v0.4 (briques nommées et paramétrables).
- Tests de vision coûteux (lancers de rayons) : limités aux gardes proches, fréquence réduite en profil DS grâce aux niveaux de détail d'IA.
- Tout réglable : difficulté de détection, indicateur au HUD, missions désactivables.
