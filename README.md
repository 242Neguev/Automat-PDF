# Automat-PDF

*Une solution développée par **Eau sèche**. Contact : +242 06 853 26 77 · gneguev@gmail.com*

Application de bureau (Windows) pour **fusionner, séparer, organiser, protéger et diffuser des PDF** : bulletins de paie séparés et nommés d'après une liste Excel puis envoyés à chacun, scans remis dans l'ordre, documents allégés, protégés ou convertis.

Automat-PDF est né du module « Outils PDF » d'Automat-Process, l'application de remplissage automatique (cartes de visite, signatures e-mail, visuels, courriers Word). Les deux applications ont été séparées le 30 octobre 2026, car elles servent des usages différents.

## Télécharger

L'installateur Windows de la dernière version se trouve sur la page [**Releases**](https://github.com/242Neguev/Automat-PDF/releases) du projet. Automat-PDF est un logiciel libre et gratuit, sous [licence MIT](LICENSE).

> L'installateur n'est pas encore signé : Windows affiche l'avertissement « Windows a protégé votre ordinateur ». Il faut cliquer sur **Informations complémentaires**, puis sur **Exécuter quand même**. Voir la [politique de signature du code](#politique-de-signature-du-code-code-signing-policy).

## Feuille de route de la première version

| Étape | Fonctions | État |
|---|---|---|
| **PDF-0** | **Fusionner** plusieurs PDF dans l'ordre voulu ; **séparer** un PDF par page ou toutes les N pages, fichiers nommés d'après une liste Excel (`{{Prénom}} {{NOM}}`) ; pages recopiées telles quelles, CMJN compris | ✅ |
| **PDF-1** | **Organiser** : pages de plusieurs PDF en vignettes, réorganisation par glisser-déposer (une page ou une sélection), pages pivotées, supprimées ou extraites, annulation, aperçu en grand ; **intercaler** deux PDF (rectos et versos scannés séparément, versos à l'envers ou non) | ✅ |
| **PDF-2** | **Images en PDF** (JPG, PNG, WEBP, TIFF, BMP, GIF ; JPEG repris sans perte, photos redressées d'après leur orientation, transparence gardée ; A4 ou page ajustée à l'image) ; **PDF en images** (PNG ou JPG, pages choisies, 96 à 300 ppp) ; **pages par feuille** : 2 ou 4 par feuille, livret à plier, format A4 (champs de formulaire remplis conservés) | ✅ |
| **PDF-3a** | **Numéros de page** (4 présentations, 6 positions, premier numéro, pages choisies) et **tampon** (« COPIE », « CONFIDENTIEL », « PAYÉ »… en filigrane transparent ou tampon encadré), avec aperçu fidèle ; **signature ou cachet** placé à la souris (déplacement, taille, flèches du clavier) sur une page, la dernière, toutes ou certaines ; à l'endroit même sur les pages tournées | ✅ |
| **PDF-3b** | **Caviardage** : zones tracées à la souris, page par page ; sous les zones, les caractères du texte (même invisible, sur une page numérisée), les pixels des images, les dessins couverts et les annotations sont supprimés du fichier ; le texte restant garde sa place ; page remplacée par son image quand un élément ne peut pas être nettoyé autrement (ou toutes, au choix) ; signets, informations et formulaires du document retirés | ✅ |
| **PDF-4** | **Mot de passe** : protéger en AES-256 (impression, copie du texte, modification permises ou non ; mot de passe du propriétaire facultatif), ou retirer le mot de passe d'un PDF dont on le connaît ; **compression** en trois niveaux (300, 200 ou 150 ppp) : chaque image ramenée à cette résolution d'après sa taille sur la page, puis en JPEG si cela allège nettement (photos, scans), sans perte sinon (dessins) ; objets en double fusionnés, vignettes et données d'édition retirées ; le fichier d'origine est gardé si rien n'est gagné | ✅ |
| **PDF-5a** | **Séparer par personne** (bulletins de paie, attestations) : texte de chaque page lu avec sa position (y compris le texte invisible des scans reconnus) ; chaque page attribuée d'après la liste (matricule ou nom, confirmé par une autre information de la personne) ou d'après un repère imprimé (« Matricule : ») ; dans le doute (récapitulatif qui cite plusieurs personnes, page sans identifiant, matricule trouvé par hasard dans un montant), la page n'est attribuée à personne et va dans « _Pages à vérifier.pdf » ; bulletin sur plusieurs pages rattaché à la page précédente au choix ; **mot de passe par personne** pris dans la liste (AES-256, impression permise, ni copie ni modification) | ✅ |
| **PDF-5b** | **Envoi de chaque PDF par Outlook** (version classique, pilotée par PowerShell ; messages transmis par un fichier temporaire, jamais par la ligne de commande) : objet et message à balises, brouillons à vérifier ou envoi direct après confirmation, essai à sa propre adresse, avancement affiché, fichiers sans adresse signalés ; **enchaînements enregistrés** dans l'application (repérage, noms, mot de passe, message, liste et colonnes choisies), repris d'un clic | ✅ |
| **PDF-6a** | **Office en PDF** : documents Word, classeurs Excel et présentations PowerPoint convertis par Microsoft Office, piloté sans l'afficher (LibreOffice en repli) ; **PDF en Word** modifiable, par Word ; fichiers protégés refusés sans bloquer, application qui ne répond plus arrêtée, raison donnée pour chaque fichier non converti, aucun fichier existant écrasé | ✅ |
| **PDF-6b** | **Clic droit dans l'Explorateur** : sous-menu « Automat-PDF » posé par l'installateur (NSIS) et retiré à la désinstallation, sur les PDF (Organiser, Fusionner, Compresser, Protéger, Séparer par personne, Convertir en Word ou en images), les images (Mettre en PDF) et les documents Office (Convertir en PDF) ; plusieurs fichiers choisis réunis dans **une seule fenêtre** ; **glisser-déposer** dans l'outil affiché, ou choix de l'outil s'il ne prend pas ces fichiers ; une liste Excel déposée sur Séparer est importée | ✅ |
| **PDF-7** | [**Fiche de test sous Windows**](docs/test-windows/README.md) : compilation, clic droit, glisser-déposer, Office, Outlook, avec des fichiers d'essai fictifs | ✅ |
| **PDF-8** | [**Premiers essais sous Windows**](docs/test-windows/README.md#premier-passage-sous-windows-4-novembre-2026) : compilation par le script, installation, clic droit, une seule fenêtre, conversions Office réelles. Corrigés : script de compilation bloqué par npm 10, plusieurs fichiers choisis l'application fermée (seul le premier arrivait), PDF en Word bloqué par le message de conversion de Word, présentations PowerPoint jamais converties (« power-point » transmis au script, qui attend « powerpoint »), PowerPoint de l'utilisateur refermé, Office laissé caché après un abandon ou une interruption du script ; LibreOffice prend désormais le relais d'une application d'Office inutilisable | ✅ |
| **PDF-9a** | **Envoi sans Outlook** : chaque PDF part de la messagerie de l'utilisateur (SMTP : Gmail, Yahoo, messagerie de l'organisation…), sous Windows comme sous macOS ; connexion chiffrée par le système (STARTTLS ou SSL/TLS), mot de passe gardé par le Gestionnaire d'identification de Windows ou le trousseau de macOS, jamais dans un fichier ; réglages des messageries courantes proposés d'après l'adresse, test de la connexion, essai à sa propre adresse ; destinataire refusé signalé sans arrêter les autres, message renvoyé quand le serveur demande d'attendre, messages non partis signalés si la connexion se perd ; Outlook reste au choix | ✅ |
| **PDF-9b** | **Office en PDF sans Office** : LibreOffice trouvé où qu'il soit installé (registre, dossiers des programmes) ; s'il manque, la page le dit avant toute conversion et propose de l'installer par son installateur officiel (winget : téléchargement, accord de l'administrateur demandé par Windows, suivi de l'installation), ou ouvre sa page de téléchargement ; **PDF en Word** prévient d'emblée quand Word manque | ✅ |
| **PDF-10** | **PDF en Word sans Word** : quand Word manque ou ne répond pas, Automat-PDF convertit lui-même, sans rien installer. Chaque page du PDF devient une page du document, au même format ; texte modifiable en paragraphes (police, taille, gras, italique, couleur, alignement, retraits, interlignes), lignes coupées par la largeur de la page remises ensemble ; colonnes d'un tableau alignées par des tabulations, cellules serrées ou tronquées d'Excel comprises, sans couper un mot ni un montant ; images reprises à leur place, filets et dessins gardés derrière le texte ; page faite seulement de dessins, ou scan sans texte reconnu, reprise en image. Documents relus par Word et LibreOffice | ✅ |
| **PDF-11** | **Édition libre** : licence MIT, dépôt public, version 0.1.0 publiée (installateur pas encore signé, demande déposée auprès de SignPath Foundation) | ✅ |
| **PDF-12** | **Prise en main sans formation** : **accueil** « Que voulez-vous faire ? » (tâches en grand ; recherche avec les mots de tous les jours : « signer », « alléger », « réunir », « bulletins de paie »… ; fichiers déposés ou choisis sur l'accueil, et l'application propose les outils qui conviennent) ; sur chaque outil, une grande zone « Glissez votre PDF ici » ; nouveau fichier proposé **à côté de l'original** ; une fois le travail fait, un encadré **Ouvrir** / **Afficher dans le dossier** (seuls les PDF, documents Word et images que l'application crée s'ouvrent ainsi) ; bouton grisé accompagné de ce qu'il manque (« Ajoutez au moins deux PDF. ») ; **Séparer** et **Séparer par personne** en étapes numérotées, colonnes de la liste ajoutées d'un clic dans les noms de fichiers et les messages, colonne du mot de passe choisie dans une liste ; vocabulaire simplifié (« Masquer des passages », « Rectos et versos », « Réglages enregistrés » ; ni AES-256, ni CMJN, ni ppp à l'écran) ; textes d'aide plus contrastés, erreurs affichées 15 secondes | ✅ |
| **PDF-13** | **Mises à jour et statistiques** : au démarrage puis chaque jour, Automat-PDF regarde sur GitHub si une nouvelle version est publiée ; un bandeau la propose, et **Mettre à jour** la télécharge, vérifie sa signature, l'installe puis rouvre l'application (recherche désactivable dans « À propos ») ; **statistiques anonymes** envoyées seulement après un « Oui » à la question posée au premier lancement (un signal par ouverture : numéro tiré au hasard, version, Windows, langue), reçues pour Eau sèche par le site de SI-Tech Global Consulting et affichées sur son tableau de bord : e-mail à chaque nouvelle installation, résumé chaque lundi avec les téléchargements et les recherches de mise à jour comptés par GitHub ([confidentialité](#confidentialité-privacy-policy)) | ✅ |

## Développer (Mac ou Windows)

Prérequis :

- [Node.js LTS](https://nodejs.org) ;
- [Rust stable](https://rustup.rs) ;
- **sur Mac** : les outils en ligne de commande Xcode (`xcode-select --install`) ;
- **sur Windows** : les *Build Tools* C++ de Visual Studio (voir plus bas).

```bash
npm install            # dépendances de l'interface
npm run tauri dev      # lance l'application en mode développement (rechargement automatique)
npm run verifier       # TypeScript, tests de l'interface, mise en forme et lints Rust, tests du cœur
```

La première compilation Rust est longue (plusieurs minutes) ; les suivantes sont rapides. En développement, l'interface est servie sur le port 1440 : Automat-Process (port 1420) peut tourner en même temps.

## Obtenir l'installateur Windows

### Option 1 : compiler sur un PC Windows (un double-clic)

1. **Récupérer le projet** de l'une de ces deux façons :
   - sur la page GitHub du dépôt, cliquer sur **Code**, puis sur **Download ZIP**, et décompresser l'archive ;
   - ou, avec Git (`winget install --id Git.Git`), lancer `git clone https://github.com/242Neguev/Automat-PDF.git`.
2. **Double-cliquer** sur `scripts\compiler-windows.cmd`. Lors de la première utilisation, le script installe ce qui manque :
   - Node.js LTS ;
   - les outils C++ de Visual Studio (plusieurs Go) ;
   - Rust.

   Windows peut demander d'autoriser ces installations. Compter de 30 à 60 minutes la première fois, quelques minutes ensuite.
3. À la fin, le dossier contenant l'installateur `.exe` s'ouvre tout seul : `src-tauri\target\release\bundle\nsis\`, ou `%LOCALAPPDATA%\Automat-PDF-compilation\target\release\bundle\nsis\` si le projet est dans OneDrive (la compilation se fait alors hors de OneDrive, qui enverrait sinon plusieurs Go de fichiers intermédiaires dans le nuage).

Pour produire aussi un `.msi`, lancer `scripts\compiler-windows.cmd -Msi` dans un terminal. Seul l'installateur `.exe` pose le sous-menu « Automat-PDF » du clic droit.

Pour essayer l'application sous Windows, suivre la [fiche de test](docs/test-windows/README.md).

> L'exécutable n'est pas encore signé : Windows affiche l'avertissement « Windows a protégé votre ordinateur ». Il faut cliquer sur **Informations complémentaires**, puis sur **Exécuter quand même**. La signature est prévue par SignPath Foundation, qui signe gratuitement les logiciels libres : voir la [politique de signature du code](#politique-de-signature-du-code-code-signing-policy).

### Option 2 : GitHub Actions (rien à installer)

Gratuit pour ce dépôt public : GitHub compile l'installateur sur l'un de ses ordinateurs Windows.

1. Ouvrir le dépôt sur GitHub, onglet **Actions**, puis **Installateur Windows** et **Run workflow**.
2. À la fin de l'exécution, télécharger les **artefacts** : l'installateur `.exe` (NSIS) et le `.msi`.

Pour publier une version, il suffit d'envoyer un tag : `git tag v0.2.0 && git push origin v0.2.0`. Une **publication GitHub en brouillon** est alors créée avec les installateurs attachés, et `latest.json`, que lisent les Automat-PDF installés : dès que la publication n'est plus un brouillon, ils proposent la mise à jour.

### Signature des mises à jour

Chaque mise à jour est signée par une clé propre au projet : avant d'installer une nouvelle version, Automat-PDF vérifie sa signature avec la clé publique écrite dans `src-tauri/tauri.conf.json`. La clé privée et son mot de passe sont rangés dans les secrets du dépôt (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) et, en lieu sûr, chez le mainteneur : perdue, elle obligerait chaque utilisateur à réinstaller Automat-PDF à la main. Seul GitHub Actions produit les fichiers de mise à jour (`src-tauri/tauri.publication.conf.json`) : une compilation sur un PC n'a pas besoin de la clé. Chaque signature porte le numéro de la version : une ancienne version ne peut pas être resservie comme une nouvelle (`requireSignedVersion`).

Quand SignPath signera l'installateur, cette signature le modifiera : il faudra refaire ensuite sa signature de mise à jour (`tauri signer sign --app-version X.Y.Z`) et la reporter dans `latest.json`, sinon les Automat-PDF installés refuseront la mise à jour.

## Organisation du code

```
src/                     Interface React + TypeScript (Mantine)
  Accueil.tsx            Accueil : « Que voulez-vous faire ? », recherche d'une tâche, zone de dépôt
  outils.tsx             Les outils, rangés par groupes : menu, accueil, fichiers reçus
  textes.ts              Tous les textes de l'interface (traduction future)
  coeur.ts               Accès typé aux commandes du cœur
  MiseAJour.tsx          Mises à jour : recherche au démarrage puis chaque jour, bandeau, téléchargement et installation
  Confidentialite.tsx    Question du premier lancement (statistiques anonymes), réglages dans « À propos »
  donnees/               Listes Excel/CSV, correspondances, contrôle des données, noms de fichiers,
                         repérage des personnes dans le texte des pages, enchaînements enregistrés,
                         réglages de la messagerie (messageries courantes reconnues d'après l'adresse),
                         ce qui manque pour convertir (Office, LibreOffice), recherche d'une tâche
                         avec les mots de tous les jours, chemins (fichier proposé à côté de l'original)
  pages/commun/reception.ts  Fichiers reçus de l'Explorateur (clic droit, glisser-déposer), confiés aux outils
  pages/                 Un dossier par outil (organiser/, mise-en-page/, fusion/, intercaler/, images/,
                         separation/, annoter/, securite/, optimiser/, convertir/), et commun/, partagé :
                         zone de dépôt, étapes numérotées, bouton d'action qui dit ce qu'il manque,
                         encadré du résultat (Ouvrir, Afficher dans le dossier), modèles à balises,
                         étape Liste et vignettes
src-tauri/               Cœur Rust (Tauri)
  src/operations.rs      Ce que fait chaque commande, sans dépendre de Tauri (testé de bout en bout)
  src/commandes.rs       Commandes appelées par l'interface, exécutées hors du fil principal
  src/pdf.rs             Fusion, séparation, assemblage de pages choisies (ordre, rotation) ; pages recopiées
                         telles quelles, liens internes vers des pages écartées retirés
  src/rendu.rs           Pages rendues en images (vignettes, aperçus, export PNG/JPG) par hayro, en Rust
  src/texte.rs           Texte des pages lu par hayro : lignes, mots et colonnes reconstitués d'après les positions
  src/envoi.rs           Messages à envoyer et bilan, communs à la messagerie et à Outlook
  src/messagerie.rs      Envoi par la messagerie (SMTP, chiffré par le système), mot de passe dans le coffre du système
  src/outlook.rs         Envoi par Outlook (Windows), par le script scripts/outlook.ps1
  src/office.rs          Office en PDF et PDF en Word, par le script scripts/office.ps1 (LibreOffice, ou
                         pour PDF en Word Automat-PDF lui-même, en repli)
  src/pdf_en_word.rs     PDF en Word sans Word : lignes, paragraphes, colonnes et images remis en page
  src/contenu.rs         Contenu des pages lu par hayro : caractères (police, taille, couleur), images
  src/docx.rs            Document Word (DOCX) écrit directement : sections, paragraphes, taquets, images
  src/libreoffice.rs     LibreOffice trouvé (registre, dossiers des programmes), ou installé par winget
  src/powershell.rs      Lancement des scripts PowerShell : données par fichier, compte rendu JSON, chien de garde
  src/images.rs          Images en PDF (JPEG repris tels quels, orientation EXIF, transparence)
  src/imposition.rs      Pages par feuille : formulaires PDF placés à l'échelle, annotations incrustées
  src/marques.rs         Surimpressions : numéros, tampon, filigrane, image (signature, cachet)
  src/caviardage.rs      Caviardage : caractères, pixels, dessins et annotations sous les zones supprimés
  src/pixels.rs          Pixels des images (8 bits) : noircis pour le caviardage, réduits pour la compression
  src/securite.rs        Mot de passe : chiffrement AES-256 et droits, ou protection retirée
  src/compression.rs     Compression : images ramenées à la résolution utile, objets en double fusionnés
  src/geometrie.rs       Matrices, cadre visible et repère de la page affichée (rotation comprise)
  src/police.rs          Helvetica standard en codage WinAnsi (accents), largeurs pour aligner
  src/memoire.rs         Derniers PDF ouverts gardés en mémoire pour les vignettes
  src/fichiers.rs        Lecture contrôlée des fichiers choisis, écriture des dossiers de résultats
  src/ouverture.rs       Fichier créé ouvert dans l'application habituelle, ou affiché dans son dossier
  src/confidentialite.rs Réglages de confidentialité ; signal d'ouverture des statistiques anonymes, avec l'accord de l'utilisateur
  scripts/               Scripts PowerShell intégrés au programme (Outlook, Office), en ASCII
  windows/clic-droit.nsh Sous-menu du clic droit, posé et retiré par l'installateur NSIS (pas par le MSI)
scripts/                 Compilation de l'installateur Windows sur un PC
```

## Licence

Automat-PDF est un logiciel libre, publié sous **licence MIT** ([LICENSE](LICENSE)) : chacun peut l'utiliser, le copier, le modifier et le redistribuer, à condition de conserver la mention de copyright et le texte de la licence. © 2026 Eau sèche.

Les bibliothèques utilisées gardent leur propre licence, toutes libres (MIT, Apache-2.0, BSD, MPL-2.0, Zlib, Unicode…).

La licence porte sur le code et les fichiers du projet. Elle ne donne pas le droit d'utiliser le nom ou le logo d'Eau sèche pour présenter un autre produit, ou une version modifiée, comme venant d'Eau sèche.

## Politique de signature du code (Code signing policy)

Les installateurs seront signés gratuitement par [SignPath Foundation](https://signpath.org), qui signe les logiciels libres, dès que le projet aura été accepté. D'ici là, ils ne sont pas signés.

*Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org).*

- Seules les versions compilées automatiquement depuis ce dépôt, par GitHub Actions, seront signées, chacune après une approbation manuelle.
- **Auteurs et relecteurs** (*committers and reviewers*) : Eau sèche ([@242Neguev](https://github.com/242Neguev)). Toute contribution proposée par une autre personne (pull request) est relue avant d'être acceptée.
- **Approbation des signatures** (*approvers*) : Eau sèche ([@242Neguev](https://github.com/242Neguev)).

*In English: Windows releases of Automat-PDF will be signed for free by [SignPath.io](https://about.signpath.io), with a certificate issued to [SignPath Foundation](https://signpath.org), once the project is accepted. Only releases built automatically from this repository by GitHub Actions, each one approved manually by the maintainer ([@242Neguev](https://github.com/242Neguev)), will be signed.*

### Confidentialité (Privacy policy)

Automat-PDF ne transmet aucune information à d'autres systèmes en réseau, sauf à la demande expresse de l'utilisateur ou de la personne qui l'installe ou l'utilise, avec une seule exception : la recherche des mises à jour, qui n'envoie rien sur l'utilisateur ni sur ses fichiers et se désactive dans « À propos ».
*This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it, with one exception: the update check, which sends no data about the user or their files and can be turned off in "About". Anonymous usage statistics are sent only if the user opts in when asked at first launch, and can be turned off at any time in "About". They are received on behalf of Eau sèche by the website of SI-Tech Global Consulting, hosted by LWS; IP addresses are not stored; each signal is kept for 13 months, and an installation's record is deleted 24 months after its last signal. To have yours deleted, write to dev@sitech-global.com.*

En pratique :

- les PDF, les listes et les documents sont traités sur l'ordinateur, sans compte ni publicité ;
- **recherche des mises à jour** (activée d'emblée, rappelée au premier lancement, désactivable dans « À propos ») : au démarrage puis une fois par jour, Automat-PDF lit sur GitHub le fichier `latest.json` de la dernière version publiée. Comme pour toute page web, GitHub voit l'adresse IP de l'ordinateur ([déclaration de confidentialité de GitHub](https://docs.github.com/fr/site-policy/privacy-policies/github-general-privacy-statement)) ; rien d'autre n'est envoyé. Une nouvelle version n'est téléchargée et installée que si l'utilisateur clique sur **Mettre à jour**, et seulement si elle porte la signature du projet ;
- **statistiques anonymes** (rien n'est envoyé tant que l'utilisateur n'a pas répondu « Oui » à la question posée au premier lancement ; réponse modifiable à tout moment dans « À propos ») : à chaque ouverture, un signal part vers Eau sèche. Il contient un numéro tiré au hasard pour l'installation, la version d'Automat-PDF, celle de Windows, la langue et l'édition ; jamais un fichier, un nom ni une adresse. Eau sèche s'en sert seulement pour savoir combien de personnes utilisent Automat-PDF, et à quelle fréquence ; ces données ne sont ni vendues ni partagées.
  - **Destinataire** : Eau sèche. Les signaux sont reçus et gardés pour son compte par le site web de SI-Tech Global Consulting ([sitech-global.com](https://www.sitech-global.com)), hébergé chez LWS.
  - **Adresse IP** : le site ne la conserve pas. Pour limiter le nombre d'envois par connexion, il n'en garde qu'une empreinte, renouvelée chaque jour, effacée au bout de deux heures et jamais rangée avec un signal. Comme pour toute page web, l'hébergeur LWS voit l'adresse IP dans ses journaux techniques, qui ne servent pas à ces statistiques ([mentions légales et confidentialité de LWS](https://www.lws.fr/a_propos_infos.php)).
  - **Durée de conservation** : chaque signal est gardé 13 mois ; la fiche d'une installation est effacée 24 mois après son dernier signal.
  - **Effacement** : écrire à [dev@sitech-global.com](mailto:dev@sitech-global.com) en donnant le numéro de l'installation, inscrit dans le fichier `%APPDATA%\com.eauseche.automatpdf\confidentialite.json` (ligne `installation`). Pour qu'aucun signal ne reparte ensuite, désactiver **Envoyer des statistiques anonymes à Eau sèche** dans « À propos » : sinon, la prochaine ouverture crée une nouvelle fiche ;
- un e-mail ne part que lorsque l'utilisateur le demande, par sa propre messagerie (le serveur qu'il a réglé) ou par Outlook ; le mot de passe de la messagerie reste dans le coffre du système (Gestionnaire d'identification de Windows, trousseau de macOS) ;
- LibreOffice n'est téléchargé, par winget, que lorsque l'utilisateur clique sur « Installer LibreOffice » et donne l'accord de l'administrateur ;
- les conversions par Microsoft Office ou LibreOffice se font sur l'ordinateur, par ces logiciels.
