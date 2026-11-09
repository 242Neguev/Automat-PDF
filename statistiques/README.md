# Statistiques d'utilisation d'Automat-PDF

Automat-PDF peut envoyer à Eau sèche un signal anonyme à chaque ouverture, **seulement si la personne l'a accepté** : la question est posée au premier lancement, et la réponse se change à tout moment dans « À propos ». Ce dossier contient le petit service, gratuit, qui reçoit ces signaux : un script Google Apps Script attaché à un classeur Google Sheets.

## Ce qui est envoyé, et rien d'autre

| Champ | Exemple | Rôle |
|---|---|---|
| `installation` | `3f2a9c1e-7b4d-4e8f-9a0b-1c2d3e4f5a6b` | numéro tiré au hasard au premier « Oui », propre à l'installation : il ne dit rien de l'ordinateur ni de la personne |
| `evenement` | `installation`, puis `ouverture` | premier signal, puis chaque ouverture |
| `version` | `0.2.1` | version d'Automat-PDF |
| `systeme` | `Windows 11 24H2 (26100)` | version de Windows |
| `langue` | `fr-FR` | langue de l'ordinateur |
| `edition` | `libre` | édition d'Automat-PDF |

Jamais de fichier, de nom de fichier, de nom de personne ni d'adresse. Google ne transmet pas au script l'adresse IP de l'expéditeur. Le code qui envoie le signal est dans [`src-tauri/src/confidentialite.rs`](../src-tauri/src/confidentialite.rs).

## Ce que vous recevez

- **Un e-mail à chaque nouvelle installation** (20 par jour au plus ; les suivantes figurent dans le résumé).
- **Un résumé chaque lundi à 8 h** : nouvelles installations, installations actives sur 7 et 30 jours, ouvertures, versions utilisées. Il ajoute les chiffres de GitHub, qui comptent **tout le monde**, y compris les personnes qui ont refusé les statistiques : téléchargements de l'installateur, et recherches de mise à jour (environ une par ouverture).
- **Un classeur Google Sheets** à consulter quand vous voulez, même sur téléphone :
  - « Synthèse » : les chiffres clés ;
  - « Installations » : une ligne par installation (première et dernière ouverture, nombre d'ouvertures et de jours d'utilisation) ;
  - « Signaux » : chaque signal reçu.

## Mise en place (une fois, 10 minutes)

Avec le compte Google qui doit recevoir les e-mails :

1. Ouvrir [sheets.new](https://sheets.new) : un classeur vide se crée. Le renommer, par exemple « Statistiques Automat-PDF ».
2. Menu **Extensions** > **Apps Script**. Dans l'éditeur qui s'ouvre, effacer le contenu de `Code.gs` et coller à la place celui du fichier [`Code.gs`](Code.gs) de ce dossier. Cliquer sur l'icône **Enregistrer** (disquette).
3. Facultatif : en haut de `Code.gs`, dans `REGLAGES`, mettre une autre adresse dans `destinataire`, ou changer le jour et l'heure du résumé.
4. Vérifier le fuseau horaire : roue dentée **Paramètres du projet** (à gauche) > **Fuseau horaire** : par exemple « (GMT+01:00) Heure d'Afrique de l'Ouest - Brazzaville ».
5. Revenir à l'éditeur (icône `< >`), choisir **installer** dans la liste des fonctions, en haut, puis cliquer sur **Exécuter**. Google demande l'autorisation : choisir le compte, puis **Paramètres avancés** > **Accéder à … (non sécurisé)** > **Autoriser**. Cet avertissement est normal pour un script que vous avez écrit vous-même et que Google n'a pas examiné. Le classeur contient ensuite les feuilles « Synthèse », « Signaux » et « Installations ».
6. **Déployer** > **Nouveau déploiement**. Roue dentée **Sélectionner le type** > **Application Web**. Description : « Automat-PDF ». **Exécuter en tant que** : Moi. **Qui a accès** : Tout le monde. Cliquer sur **Déployer**, puis copier l'**URL de l'application Web** (elle se termine par `/exec`).
7. Vérifier : ouvrir cette adresse dans le navigateur. La page doit afficher « Automat-PDF : le service des statistiques est en marche. »
8. Donner l'adresse à GitHub, qui l'intègre aux versions qu'il compile : dépôt **Automat-PDF** > **Settings** > **Secrets and variables** > **Actions** > **New repository secret**. Nom : `AUTOMAT_PDF_STATISTIQUES` ; valeur : l'adresse copiée.

Les versions compilées ensuite par GitHub (étiquette `vX.Y.Z`) poseront la question au premier lancement. Les copies compilées sans ce secret (sur un PC, ou par d'autres) ne la posent pas et n'envoient jamais rien.

## Modifier le script plus tard

Après une modification de `Code.gs` : **Déployer** > **Gérer les déploiements** > icône crayon > **Version** : « Nouvelle version » > **Déployer**. L'adresse ne change pas : rien à refaire sur GitHub. Un « Nouveau déploiement », lui, créerait une autre adresse.

## Limites du service gratuit

Un compte Google gratuit permet 100 e-mails par jour à un script, et un classeur peut contenir 10 millions de cellules : environ un million de signaux. Au-delà, copier la feuille « Signaux » ailleurs, puis la vider en gardant sa première ligne.

## Effacer des données

Pour effacer les données d'une installation, il suffit de supprimer ses lignes dans « Installations » et dans « Signaux » (filtre sur la colonne « Installation »). Si la personne laisse les statistiques activées, la prochaine ouverture recrée sa ligne, comptée comme une nouvelle installation.
