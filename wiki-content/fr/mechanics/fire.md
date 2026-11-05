## Principe

Le **feu** est un bloc de flammes qui éclaire (niveau 15), vieillit et se propage. Il tient sur un bloc solide ou à côté d'un bloc inflammable. À chaque pas (un peu plus d'une seconde et demie), il prend de l'âge, peut **consumer** un voisin inflammable (parfois en lui laissant une flamme) et **allumer l'air** autour de lui, jusqu'à quatre blocs au-dessus, quand cet air touche quelque chose qui brûle. Plus il est vieux, moins il se propage ; un feu sans combustible s'éteint vite, et tout incendie finit par mourir de lui-même.

## Le briquet (créatif)

Dans l'onglet **Outils** de l'inventaire créatif, le **briquet** allume un feu d'un **clic droit** sur la face visée, si un feu peut tenir à cet endroit (pas à côté de l'eau). Le serveur vérifie chaque usage (portée, place libre), comme pour poser un bloc.

## Ce qui brûle

- **Bûches** : s'enflamment difficilement et se consument lentement.
- **Planches** : un peu plus vite.
- **Feuilles** : prennent facilement.
- **Herbes, fougères, buissons, fleurs** : partent en un instant.
- **Lianes** : brûlent vite mais allument peu l'air autour.

La pierre, la terre, le sable et les champignons ne brûlent pas. Le wiki de chaque bloc indique s'il est **inflammable**.

## Éteindre

- L'**eau** éteint aussitôt un feu qui la touche, et aucun feu ne s'allume à côté d'elle.
- Casser le feu ou poser un bloc à sa place l'efface.
- Sans combustible, il s'éteint seul en quelques secondes.

## Réglages

Options de monde, choisies à la création ou avec `/fire` : `spread` (le feu allume de nouveaux feux) et `destroy` (ce qui brûle est détruit). Les feux partagent le budget de mises à jour par tick des liquides (`/fluids <budget>`).
