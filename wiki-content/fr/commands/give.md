## Description

`/give` dépose un objet directement dans la case choisie de votre barre. Plus rapide que de fouiller l'inventaire créatif quand on connaît déjà l'identifiant de ce qu'on veut.

## Syntaxe

`/give <objet>`, où l'objet est un identifiant :
- complet, avec son espace de noms : `strata:stone` ;
- ou court, pour le contenu de base : `stone` équivaut à `strata:stone`.

## Exemples

- `/give glass` pour une vitre toute fraîche.
- `/give strata:fir_log` pour une bûche de sapin.

## Comportement

Exécutée par le serveur, en **mode créatif** uniquement. L'objet **remplace** ce qui occupait la case sélectionnée. Un identifiant inconnu est signalé, sans rien changer. Certains blocs n'ont pas d'objet (l'eau, par exemple) et ne peuvent donc pas être donnés.

## Conseils

Appuyez sur Tab pendant la saisie : la console complète les identifiants d'objets. Pratique pour découvrir le nom exact d'un bloc.

## Découverte en jeu

`/help give` rappelle la syntaxe. Pour connaître l'identifiant d'un bloc, visez-le avec F3 activé : la ligne « bloc visé » l'affiche.
