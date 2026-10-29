# Pêche — mise à jour dédiée

**Statut : À valider.** Version proposée : v1.2.

Objectif : faire de la pêche de la v0.5 un système complet, avec sa progression, sa collection et ses secrets, jouable seul comme une activité à part entière.

## Matériel

- **Cannes par tiers et améliorations (décidé)** : une canne par matériau (bois, fer, diamant, puis matériaux des dimensions) pour la portée du lancer, la solidité et la vitesse de remontée ; **moulinets, fils et hameçons** interchangeables en plus.
- **Pas de niveau de pêche (décidé)** : le joueur ne gagne aucune habileté, tout passe par le matériel, comme le reste du jeu.
- **Pêche dans la lave (décidé)** : possible dans l'Infernum avec une canne résistante au feu (Ignium ou équivalent), espèces de lave dédiées.
- **Appâts** (vers, insectes, pâtes, appâts cuisinés) et **leurres** (cuillers, mouches, leurres lumineux pour les profondeurs) : chacun attire certaines espèces.
- **Sacoche de pêche** pour appâts et prises, cohérente avec les sacoches spécialisées existantes.
- **Pêche sous la glace** : percer la glace avec un outil dédié, abri de pêche.
- **Pêche en bateau** et pêche en profondeur avec lest.
- **Abyssus (décidé)** : pour pêcher dans chaque palier de profondeur (voir `abyssus.md`), il faut un fil et un lest assez solides pour l'atteindre.

## Espèces

- **120 espèces et plus (décidé)**, façon *Animal Crossing* : environ un tiers sur Terra, le reste réparti dans les dimensions.
- Chaque poisson a ses conditions d'apparition : **biome, saison, heure, météo, profondeur, type d'eau** (rivière, lac, mer, grotte, marais) et **dimension**.
- Les dimensions ont leurs propres espèces : poissons de lave dans l'Infernum (canne résistante au feu), poissons des abysses dans l'Abyssus, poissons de brume dans Crepuscula, poissons de nuages dans Machina Caeli.
- Les événements célestes (lune de sang, éclipse, étoiles filantes) font apparaître des espèces rares.
- **Taille et poids variables** par prise, notés au journal (record personnel). Pas de rareté aléatoire sur les objets : la variation reste une donnée de la prise, pas une qualité d'équipement.
- Objets repêchés : déchets, trésors, bouteilles à message, pages de recettes, fragments de journaux abandonnés.

## Mini-jeu

- **Décidé : barre de tension**, façon *Stardew Valley*.
- **Ferrage** au bon moment, puis **combat** avec le poisson : tension du fil à maintenir dans une zone.
- **Chaque espèce se comporte différemment (décidé)** : force et vitesse de traction, comportement (plongées, sauts, feintes), endurance selon la taille.
- Jouable à la souris, au clavier et à la manette (gâchettes pour la tension, vibration si disponible).
- Réglable : difficulté du mini-jeu, ou mode simplifié façon Minecraft pour qui n'en veut pas.

## Savoir et collection

- **Carnet de pêche** dans le journal : espèces découvertes, conditions connues, record de taille, appât préféré. Les conditions se révèlent en pêchant, comme le reste du savoir.
- PNJ pêcheurs qui donnent des indices et des rumeurs de poissons légendaires.
- Progrès dédiés (première prise, collection complète par biome, par dimension).

## Poissons légendaires

- Un poisson unique et nommé par grande zone ou dimension, chacun avec des conditions précises et un mini-jeu de pêche très difficile.
- **Prises rares, pas des boss** : aucun combat hors du mini-jeu de pêche.
- Non farmables. Noms en latin marqués `[NOM À VALIDER]`, textes `[LORE À ÉCRIRE]`.

## Valorisation

- **Cuisine** : recettes de poisson liées au système de cuisine de la v0.5. **Certains poissons cuisinés donnent des effets (décidé)** : respiration, résistance au froid, vision nocturne… à découvrir. La règle « même effet non cumulable » s'applique avec les potions et les sorts.
- **Aquariums** en blocs, poissons vivants animés ; trophées muraux pour les grosses prises.
- **Élevage complet (décidé)** : bassins et viviers, reproduction des poissons, production de poissons. Les poissons légendaires ne se reproduisent pas.
- **Pisciculture automatisable (décidé)** : nourrisseurs et collecteurs mécaniques. Seule exception au principe « pêche sans voie » : la prise à la canne reste manuelle.
- **Commerce** avec les villageois pêcheurs.
- **Concours de pêche** pendant les fêtes de village saisonnières. **Récompenses (décidé)** : pièces et objets, matériel exclusif (leurres, pièces de canne uniques), réputation auprès du village, trophées à exposer.

## Pas de voies

**Décidé :** la pêche est une activité à part, **sans voie magique ni mécanique**. Tout passe par la canne, le matériel, l'élevage et le savoir du joueur.

## Technique et performances

- Espèces, conditions, appâts et butins définis en packs RON.
- Poissons visibles dans l'eau (bancs) avec niveau de détail : simplifiés ou masqués en profil DS.
- Tout le système désactivable ou réglable (mini-jeu, poissons visibles, nombre d'espèces affichées).
