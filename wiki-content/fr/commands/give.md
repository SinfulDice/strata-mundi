## Description

`/give` ajoute des objets à votre inventaire. Plus rapide que de fouiller l'inventaire créatif quand on connaît déjà l'identifiant de ce qu'on veut.

## Syntaxe

`/give <objet> [nombre]`, où l'objet est un identifiant :
- complet, avec son espace de noms : `strata:stone` ;
- ou court, pour le contenu de base : `stone` équivaut à `strata:stone`.

Le nombre vaut 1 s'il est omis.

## Exemples

- `/give glass` pour une vitre toute fraîche.
- `/give strata:fir_log 128` pour deux piles de bûches de sapin.

## Comportement

Exécutée par le serveur, dans les mondes où les commandes de triche sont permises. Les objets rejoignent les piles existantes puis les cases libres (barre d'abord) ; ce qui ne tient pas tombe au sol devant vous. Un identifiant inconnu est signalé, sans rien changer. Certains blocs n'ont pas d'objet (l'eau, par exemple) et ne peuvent donc pas être donnés.

## Conseils

Appuyez sur Tab pendant la saisie : la console complète les identifiants d'objets. Pratique pour découvrir le nom exact d'un bloc.

## Découverte en jeu

`/help give` rappelle la syntaxe. Pour connaître l'identifiant d'un bloc, visez-le avec F3 activé : la ligne « bloc visé » l'affiche.
