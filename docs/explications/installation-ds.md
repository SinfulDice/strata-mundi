# Installation sur le DS (Linux maison)

*Ce fichier est complété au jalon 8 (dépendances système complètes : Vulkan Mesa Intel, audio, entrées).*

## Fermeture du clapet

### À quoi ça sert

Refermer le DS doit mettre la partie en pause et la sauvegarder tout de suite ; le rouvrir reprend la partie là où elle était, sans menu à traverser.

### Comment ça marche

Sous Linux, c'est **logind** (le gestionnaire de sessions de systemd, ou `elogind` sur un système sans systemd) qui met la machine en veille. Juste avant, il envoie le signal D-Bus `PrepareForSleep(true)` ; au réveil, `PrepareForSleep(false)`.

1. Au démarrage, le jeu prend un **verrou de retardement** (*delay inhibitor lock*) : logind attendra que le jeu le relâche avant de mettre en veille, au plus `InhibitDelayMaxSec` secondes (5 par défaut).
2. Clapet fermé → `PrepareForSleep(true)` → le jeu met le monde en pause, demande une sauvegarde (avec une capture pour la liste des mondes), attend la confirmation du serveur (4 s au plus), puis relâche le verrou : la machine s'endort.
3. Clapet ouvert → `PrepareForSleep(false)` → la partie reprend et le jeu reprend un verrou pour la prochaine fois.

Sans logind (système sans D-Bus, conteneur), le jeu le note dans son journal et continue normalement : seule la sauvegarde automatique (toutes les 5 minutes) protège alors la partie.

### Ce qu'il faut sur le système

- **D-Bus système** et **logind** (`systemd-logind` ou `elogind`) qui gère la mise en veille (pour le clapet : `HandleLidSwitch=suspend` dans `/etc/systemd/logind.conf` ou `/etc/elogind/logind.conf`).
- Si 5 secondes ne suffisent pas pour sauvegarder (gros monde sur une carte SD lente), augmenter `InhibitDelayMaxSec` dans le même fichier.
- Pour vérifier : `busctl monitor org.freedesktop.login1` puis fermer le clapet ; le signal `PrepareForSleep` doit apparaître.

## Où c'est dans le code

- `crates/client/src/sleep.rs` (`SleepWatcher`) : écoute logind sur un fil d'exécution séparé, avec la crate `zbus`.
- `crates/client/src/session.rs` (`watch_sleep`) : pause, sauvegarde, puis feu vert.

<details><summary>Côté Rust</summary>

- **`#[cfg(target_os = "linux")]`** : ce code n'est compilé que sous Linux ; sous Windows, une version vide le remplace et le jeu fonctionne sans.
- **Canaux `mpsc`** : le fil d'écoute envoie « on s'endort » au fil principal, qui répond « c'est sauvegardé » par un second canal.
- **`OwnedFd`** : le verrou de logind est un descripteur de fichier ; le **relâcher** consiste simplement à le fermer (`drop`).

</details>
