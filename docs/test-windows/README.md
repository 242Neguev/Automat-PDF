# Fiche de test sous Windows

Tout ce qui ne peut pas être essayé sur Mac : la compilation et l'installateur, le clic droit dans l'Explorateur, la fenêtre unique, le glisser-déposer depuis l'Explorateur, les conversions par Microsoft Office et l'envoi par e-mail (par votre messagerie ou par Outlook). Le reste de l'application a été testé de bout en bout sur Mac, avec le vrai cœur de traitement.

**Version à tester** : le commit `9d5264d` ou plus récent (`git log -1` pour le vérifier).

## Avant de commencer

- Un PC sous Windows 10 ou 11, avec Internet : la première compilation télécharge plusieurs Go (moins si Automat-Process a déjà été compilé sur ce PC).
- Microsoft Office : Word, Excel et PowerPoint (parties 5 et 6), Outlook classique (partie 8b). Le nouvel Outlook, s'il est installé, est un plus. Un PC sans Office est utile aussi : LibreOffice s'y installe depuis l'application (partie 5), et Automat-PDF y convertit lui-même les PDF en Word (partie 6).
- Une adresse e-mail et son mot de passe (partie 8a) : Gmail, Yahoo, ou celle de votre organisation. Gmail et Yahoo demandent un « mot de passe d'application », à créer avant l'essai dans la sécurité du compte.
- Environ 1 h 30 la première fois, dont 30 à 60 minutes de compilation.
- Créer un dossier d'essai dont le nom contient un espace et un accent, par exemple `Documents\Essai Automat été`, et y copier les fichiers d'essai : tous les fichiers produits iront dedans.

**Fichiers d'essai** (fictifs) dans [`fichiers/`](fichiers) :

| Fichier | Sert à |
|---|---|
| `Bulletins octobre.pdf` | 9 pages de paie : 6 salariés, dont deux sur deux pages, et un récapitulatif qui les cite tous (parties 2, 3, 6, 7) |
| `personnel-essai.csv` | liste des salariés, à compléter avec **votre** adresse Outlook (parties 7 et 8) |
| `Scan courrier.pdf` | une page numérisée à 200 ppp, 1,8 Mo (parties 2 et 4) |
| `Convocation formation.docx`, `Liste des agents.xlsx` | documents Office à convertir (parties 2, 3 et 5) |
| `Photo portrait.jpg` | photo prise « couchée » : elle doit ressortir droite, le mot HAUT en haut (parties 2 et 4) |
| `Cachet.png` | cachet rond à fond transparent (partie 4) |

**En cas de problème** : faire une capture d'écran (touches `Windows + Maj + S`), recopier le message exact et noter l'action qui l'a provoqué. Puis passer à la suite : les parties sont indépendantes, sauf la 1.

## 1. Compiler et installer

1. Récupérer la dernière version : `git pull`, ou télécharger à nouveau le ZIP depuis GitHub.
2. Double-cliquer sur `scripts\compiler-windows.cmd`, **sans** l'option `-Msi` : le clic droit n'est posé que par l'installateur ordinaire.
   - Accepter les installations proposées : Node.js, outils C++ de Visual Studio, Rust.
   - Si le script demande de fermer la fenêtre et de le relancer (après une installation), le faire.
   - S'il s'arrête sur **ÉCHEC**, faire une capture des 30 dernières lignes de la fenêtre.
3. À la fin, le dossier de l'installateur s'ouvre : `src-tauri\target\release\bundle\nsis\`, ou `%LOCALAPPDATA%\Automat-PDF-compilation\target\release\bundle\nsis\` si le projet est dans OneDrive. Lancer `Automat-PDF_0.2.2_x64-setup.exe`.
   - « Windows a protégé votre ordinateur » : cliquer sur **Informations complémentaires**, puis sur **Exécuter quand même**.
   - L'installation se fait pour l'utilisateur courant, sans droits d'administrateur.

**Attendu** : l'application démarre sur l'**accueil** « Que voulez-vous faire ? » : une recherche, une zone « Glissez vos fichiers ici » et les 15 tâches en grand. Le menu compte **Accueil**, puis les 15 outils en 7 groupes, de **Organiser** à **PDF en Word**, et défile si la fenêtre est petite. Le bouton ⓘ, en haut à droite, affiche Eau sèche et ses coordonnées.

À noter aussi : le temps de compilation, et si l'antivirus a réagi.

## 2. Clic droit dans l'Explorateur

Sous Windows 11, le menu du clic droit est abrégé : cliquer d'abord sur **Afficher plus d'options** (ou faire `Maj + clic droit`).

1. **Fermer Automat-PDF.** Dans le dossier d'essai, clic droit sur `Scan courrier.pdf` → **Automat-PDF** → **Compresser**.
   **Attendu** : le sous-menu propose, dans l'ordre, Organiser les pages, Fusionner, Compresser, Protéger par un mot de passe, Séparer par personne, Convertir en Word, Convertir en images. L'application s'ouvre sur **Compresser**, avec « Scan courrier.pdf · 1,8 Mo ».
2. **Sans fermer l'application**, sélectionner `Bulletins octobre.pdf` et `Scan courrier.pdf`, clic droit → **Automat-PDF** → **Fusionner**.
   **Attendu** : aucune seconde fenêtre ; la fenêtre déjà ouverte passe au premier plan, sur **Fusionner**, avec les deux fichiers (« 2 fichiers, 10 pages au total »).
3. Clic droit sur `Photo portrait.jpg` → **Automat-PDF** → **Mettre en PDF**.
   **Attendu** : l'outil **Images en PDF** montre la photo droite, le mot HAUT en haut.
4. Clic droit sur `Convocation formation.docx` → **Automat-PDF** → **Convertir en PDF**.
   **Attendu** : l'outil **Office en PDF** liste le document.
5. **Fermer Automat-PDF**, puis sélectionner à nouveau `Bulletins octobre.pdf` et `Scan courrier.pdf`, clic droit → **Automat-PDF** → **Fusionner**.
   **Attendu** : une seule fenêtre s'ouvre, sur **Fusionner**, avec les deux fichiers (chaque fichier lance l'application ; le second arrive pendant que la fenêtre se charge).

## 3. Glisser-déposer

1. Afficher **Organiser**, puis faire glisser `Bulletins octobre.pdf` depuis l'Explorateur sur la fenêtre.
   **Attendu** : pendant le survol, un cadre en pointillés annonce « Déposez les fichiers : ils iront à « Organiser » » ; au dépôt, les 9 pages apparaissent.
2. Afficher **Compresser**, puis y faire glisser `Liste des agents.xlsx`.
   **Attendu** : une fenêtre « Que faire de ce fichier ? » propose **Séparer**, **Séparer par personne** et **Office en PDF**.
3. Afficher l'**Accueil**, puis y faire glisser `Scan courrier.pdf`.
   **Attendu** : « Que faire de ce fichier ? » propose les 13 outils qui prennent un PDF, chacun avec sa description ; **Compresser** ouvre l'outil avec le fichier.
4. Afficher l'**Accueil**, puis y faire glisser `Convocation formation.docx`.
   **Attendu** : un seul outil convient : **Office en PDF** s'ouvre directement, avec le document.

## 4. Outils PDF, en bref

Ils ont tous été essayés sur Mac ; il s'agit de vérifier qu'ils se comportent de même sous Windows, avec des chemins accentués.

1. **Compresser** `Scan courrier.pdf` en **Forte**.
   **Attendu** : la fenêtre d'enregistrement s'ouvre **dans le dossier d'essai**, avec le nom « Scan courrier - compressé.pdf ». Une fois enregistré, un encadré vert « PDF compressé enregistré » donne « 1,8 Mo → 175 Ko » environ (−90 % au moins) et le dossier. **Ouvrir** affiche le PDF dans le lecteur habituel (Edge…) : la page compressée reste lisible. **Afficher dans le dossier** ouvre l'Explorateur sur le dossier d'essai, le fichier sélectionné.
2. **Images en PDF** avec `Photo portrait.jpg` : **Créer le PDF…**.
   **Attendu** : une page en portrait, la photo droite.
3. **Signature ou cachet** : `Bulletins octobre.pdf`, puis l'image `Cachet.png`, placée sur la dernière page.
   **Attendu** : le cachet rond, sans carré blanc autour.
4. **Mot de passe** : protéger `Bulletins octobre.pdf`, puis ouvrir le PDF protégé dans Edge.
   **Attendu** : Edge demande le mot de passe et ouvre le fichier avec.
5. **Organiser** : tourner une page, en déplacer une autre, puis **Enregistrer le PDF…**. Ouvrir le résultat par le bouton **Ouvrir** de l'encadré.
6. **Accueil** : taper « alléger » dans la recherche, puis « mot de passe », puis « zorglub ».
   **Attendu** : seule la tâche **Compresser**, puis seule **Mot de passe** ; enfin « Rien ne correspond à « zorglub »… », avec des exemples de mots.
7. **Fusionner**, sans aucun fichier.
   **Attendu** : le bouton **Fusionner…** est grisé, avec « Ajoutez au moins deux PDF. » à côté ; après un seul PDF : « Ajoutez au moins un autre PDF. ».

## 5. Office en PDF

1. **Office en PDF** : **Ajouter des fichiers…** `Convocation formation.docx`, `Liste des agents.xlsx` et **une présentation PowerPoint à vous**. **Convertir…** dans le dossier d'essai.
   **Attendu** : une barre d'avancement, aucune fenêtre d'Office à l'écran, puis « 3 fichiers convertis ». Les trois PDF ont la même mise en page que les documents d'origine (polices, marges, tableau).
2. Recommencer **avec Word déjà ouvert** sur un autre document.
   **Attendu** : ce document reste ouvert et intact ; les nouveaux PDF s'appellent « Convocation formation (2).pdf »… sans écraser les premiers.
3. Ouvrir ensuite le Gestionnaire des tâches (`Ctrl + Maj + Échap`).
   **Attendu** : ni Word, ni Excel, ni PowerPoint ne restent en arrière-plan, à part le Word ouvert à la main.
4. Si vous avez un document Word protégé par un mot de passe, l'ajouter et convertir.
   **Attendu** : il est indiqué « non converti », avec la raison, sans bloquer les autres.
5. Ouvrir une présentation dans PowerPoint, puis convertir à nouveau la présentation de l'étape 1.
   **Attendu** : PowerPoint et la présentation ouverte restent là, intacts (PowerPoint n'a qu'une fenêtre pour tout le monde : l'application s'en sert sans la refermer).

**Si Office ne répond pas pendant 5 minutes** : l'application l'arrête, sans laisser Word, Excel ou PowerPoint caché en arrière-plan, et convertit le reste sans lui. Office travaille sur des copies : vos documents ne sont jamais marqués par Office comme ayant « provoqué une erreur grave ». LibreOffice prend le relais s'il est installé ; sinon, le document est indiqué « non converti », avec la raison (« Microsoft Word ne répond pas… »). Ouvrir alors Word, Excel et PowerPoint une fois à la main, fermer leurs éventuelles fenêtres de premier lancement (activation, confidentialité), puis réessayer. Noter ce qui s'affichait.

**Office dont la licence a expiré** (« Produit sans licence », « Activer Office ») : piloté par l'application, il peut mettre **3 minutes** à démarrer ; patienter. Une application d'Office installée mais impossible à piloter est traitée comme absente : LibreOffice prend le relais.

**Sur un PC sans Office ni LibreOffice**, si vous en avez un :
1. Ouvrir **Office en PDF**.
   **Attendu** : « Ni Microsoft Office ni LibreOffice ne sont installés », avec **Installer LibreOffice** et **Page de téléchargement**.
2. **Installer LibreOffice**, confirmer, puis accepter l'accord de l'administrateur que demande Windows (sa fenêtre peut attendre dans la barre des tâches).
   **Attendu** : la durée de l'installation s'affiche ; après quelques minutes, « LibreOffice est installé » et l'avertissement disparaît. L'essai 5.1 se fait alors par LibreOffice. Noter la durée.
3. Ouvrir **PDF en Word**.
   **Attendu** : « Microsoft Word n'est pas installé sur ce PC… », dès l'ouverture de la page.

## 6. PDF en Word

1. **PDF en Word** : **Ajouter des fichiers…** `Bulletins octobre.pdf`, puis **Convertir…** dans le dossier d'essai.
   **Attendu** : `Bulletins octobre.docx`, qui s'ouvre dans Word, avec du texte modifiable et une mise en page proche du PDF.
2. **Attendu** : aucune fenêtre de Word ne s'affiche. Le message « Word va maintenant convertir votre PDF… », qui bloquerait Word caché, est évité : l'option « Ne plus afficher ce message » est posée le temps de la conversion, puis remise comme elle était. Noter combien de temps la conversion a pris.
3. **Sur un PC sans Word**, si vous en avez un : convertir `Bulletins octobre.pdf` et `Scan courrier.pdf`.
   **Attendu** : la page prévient qu'Automat-PDF convertira lui-même ; l'avancement compte les fichiers un à un et, quelques secondes plus tard, sans rien installer, « 2 fichiers convertis ». `Bulletins octobre.docx` compte 9 pages A4, comme le PDF : texte modifiable dans sa police, tableaux en colonnes alignées par des tabulations, filets du tableau derrière le texte, et aucun mot coupé dans le récapitulatif de la page 9. `Scan courrier.docx` reprend la page numérisée en image. Ouvrir les deux dans Word ou dans LibreOffice Writer et les comparer aux PDF.

## 7. Bulletins de paie : séparer par personne

1. Ouvrir `personnel-essai.csv` avec le Bloc-notes. *Édition > Remplacer* : remplacer `votre.adresse@entreprise.com` par **votre** adresse Outlook, puis **Remplacer tout**. Enregistrer.
2. **Séparer par personne** : à l'étape 1, **Choisir le PDF…** `Bulletins octobre.pdf` ; à l'étape 2, **Importer la liste…** `personnel-essai.csv`.
   **Attendu** : à l'étape 3, sous le nom des fichiers, « Exemple : Bulletin NKOULOU MBARGA Jean-Baptiste.pdf ». La vérification (étape 4) indique « 6 fichiers · 7 pages attribuées » et 2 pages à vérifier : la page 7 (sans identifiant reconnu) et la page 9 (plusieurs personnes possibles : c'est le récapitulatif).
3. Activer **Une page où personne n'est reconnu suit la précédente**.
   **Attendu** : « 8 pages attribuées » ; seule la page 9 reste à vérifier.
4. Activer **Protéger chaque fichier par un mot de passe propre à la personne**, puis **Colonne du mot de passe** : **Date de naissance**.
   **Attendu** : la colonne « Mot de passe » de la vérification indique « défini » partout ; dans le tableau de l'étape 2, la ligne **Mot de passe** est reliée à **Date de naissance**.
5. **Créer les fichiers…** : la fenêtre s'ouvre dans le dossier d'essai ; **Sélectionner un dossier**.
   **Attendu** : l'encadré « 7 fichiers créés » signale `_Pages à vérifier.pdf` et propose **Ouvrir le dossier**, qui ouvre dans l'Explorateur un dossier `Bulletins octobre - par personne` avec 6 PDF et `_Pages à vérifier.pdf`. `Bulletin DIALLO Aïssatou.pdf`, ouvert dans Edge, demande un mot de passe : `03/11/1987` l'ouvre, sur une page. `Bulletin OKEMBA Grâce.pdf` (`08/09/1990`) a deux pages.

## 8. Envoi par e-mail

### 8a. Par votre messagerie, sans Outlook

1. Même écran : activer **Envoyer chaque fichier par e-mail**. Objet : « Votre bulletin de paie d'octobre 2026 ». **Envoyer avec** : **Ma messagerie**.
   **Attendu** : « Réglez d'abord votre messagerie… ».
2. **Régler la messagerie…**, puis saisir votre adresse.
   **Attendu** : pour Gmail ou Yahoo, le serveur, le port et le chiffrement se remplissent d'eux-mêmes, avec un conseil sur le « mot de passe d'application ». Pour une autre adresse, saisir le serveur d'envoi (SMTP) donné par votre messagerie.
3. Saisir un mot de passe faux, puis **Tester la connexion**.
   **Attendu** : « Le serveur refuse l'identifiant ou le mot de passe… », avec la réponse du serveur.
4. Saisir le bon mot de passe (d'application si demandé), **Tester la connexion**, puis **Enregistrer**.
   **Attendu** : « Connexion réussie », puis « Depuis votre adresse, par le serveur ». Le mot de passe n'est écrit dans aucun fichier : il figure dans le Gestionnaire d'identification de Windows (Panneau de configuration → Gestionnaire d'identification → Informations d'identification Windows), à « messagerie.Automat-PDF ».
5. Si l'écran le demande, **Créer les fichiers…** à nouveau. Puis **Envoyer l'essai** : sans autre adresse saisie, il part à votre adresse.
   **Attendu** : le message arrive, de votre adresse (avec le nom affiché s'il a été saisi), avec `Bulletin NKOULOU MBARGA Jean-Baptiste.pdf` en pièce jointe et le texte « Bonjour Jean-Baptiste, … » sur plusieurs lignes, accents intacts. La pièce jointe s'ouvre avec le mot de passe de la personne.
6. **Envoyer 5 e-mails…** et confirmer.
   **Attendu** : la confirmation dit « depuis » votre adresse ; les 5 messages arrivent, chacun avec le bulletin de la bonne personne.
7. Couper le Wi-Fi (ou débrancher le câble), puis refaire l'essai (étape 5).
   **Attendu** : « Impossible de joindre le serveur… », sans blocage. Rebrancher.

À noter : le temps d'envoi des 5 messages, et tout message du serveur (limite d'envoi, message refusé comme indésirable). Avec Microsoft 365, l'administrateur doit souvent autoriser l'envoi SMTP authentifié : noter le message obtenu.

### 8b. Par Outlook

Outlook classique doit être ouvert.

1. Même écran : activer **Envoyer chaque fichier par e-mail**. Objet : « Votre bulletin de paie d'octobre 2026 ». **Envoyer avec** : **Outlook (version classique)**. Garder **Brouillons à vérifier**.
2. Si l'écran le demande, **Créer les fichiers…** à nouveau.
   **Attendu** : « 5 messages prêts » et « 1 fichier sans adresse, non envoyé : Bulletin NGOMA Paul ». Le contrôle de la liste signale « Personne en double (même E-mail) » : c'est normal ici, tous les messages vous étant adressés.
3. **Adresse pour un essai** : votre adresse, puis **Préparer un brouillon d'essai**.
   **Attendu** : dans les Brouillons d'Outlook, un message pour vous, avec `Bulletin NKOULOU MBARGA Jean-Baptiste.pdf` en pièce jointe et le texte « Bonjour Jean-Baptiste, … » sur plusieurs lignes. L'envoyer à la main, le recevoir, et ouvrir la pièce jointe avec `12/04/1980`.
4. **Préparer 5 brouillons dans Outlook**.
   **Attendu** : 5 brouillons, chacun avec le bulletin de la bonne personne.
5. Choisir **Envoi direct**, puis **Envoyer 5 e-mails…** et confirmer.
   **Attendu** : les 5 messages arrivent dans votre boîte de réception.
6. Fermer Outlook et refaire l'essai (étape 3).
   **Attendu** : le message « Outlook n'est pas ouvert : ouvrez Outlook (la version classique, pas le « nouvel Outlook »), puis recommencez. »

À noter en particulier : une fenêtre de sécurité d'Outlook (« Un programme tente d'envoyer un message… ») ou une alerte de l'antivirus au sujet de PowerShell. Recopier son texte exact.

**Nouvel Outlook** (s'il est installé à la place) : noter le message obtenu.

## 9. Réglages enregistrés

1. Toujours dans **Séparer par personne** : **Enregistrer ces réglages…**, nom « Bulletins de paie », **Enregistrer**.
2. Fermer l'application et la rouvrir. **Séparer par personne** → **Réglages enregistrés** : choisir « Bulletins de paie ».
   **Attendu** : les réglages reviennent, et la liste `personnel-essai.csv` est rouverte d'elle-même. Après **Choisir le PDF…** `Bulletins octobre.pdf`, les mots de passe sont « définis » sans rien rechoisir.
3. **Supprimer**, puis confirmer.
   **Attendu** : la liste des réglages enregistrés est vide (« Aucun réglage enregistré pour l'instant »).

## 10. Mises à jour et statistiques

À faire avec un installateur compilé par GitHub (étiquette `vX.Y.Z`) : seul celui-là peut envoyer des statistiques. Elles arrivent sur le site de Sitech, dans l'administration : **Logiciels › Vue d'ensemble**, bloc « Utilisation » de la carte Automat-PDF ; **Logiciels › Utilisation** retrouve une installation par son numéro.

1. Premier lancement après l'installation.
   **Attendu** : la question « Aider Eau sèche à améliorer Automat-PDF ? », avec « Me prévenir quand une nouvelle version est disponible » coché ; rien d'autre ne s'affiche tant qu'on n'a pas répondu. Choisir **Oui, j'accepte**.
2. Quelques minutes plus tard, dans la vue d'ensemble du site.
   **Attendu** : une nouvelle installation et une ouverture ; un e-mail « nouvelle installation » à dev@sitech-global.com. Fermer puis rouvrir Automat-PDF : 2 ouvertures, sans nouvel e-mail.
3. **À propos** (en haut à droite) : **Rechercher une mise à jour**.
   **Attendu** : « Vous avez la dernière version. » Désactiver **Envoyer des statistiques anonymes à Eau sèche**, fermer puis rouvrir Automat-PDF : aucune ouverture de plus sur le site.
4. Quand une version plus récente est publiée sur GitHub (publication qui n'est plus un brouillon) : lancer l'ancienne.
   **Attendu** : au bout de quelques secondes, le bandeau « Une nouvelle version d'Automat-PDF est disponible ». **Mettre à jour** : barre de téléchargement, puis une petite fenêtre d'installation sans question, puis Automat-PDF se rouvre, avec la nouvelle version dans **À propos**. Les réglages enregistrés et le clic droit sont toujours là.

**En mode développement** (`npm run tauri dev`), l'application n'envoie des statistiques que si la variable `AUTOMAT_PDF_STATISTIQUES` contient l'adresse du site au moment de la compilation. Ses signaux portent alors l'édition « essai » : le site les enregistre sans jamais les compter (« Dernier signal d'essai » dans la vue d'ensemble) et les efface au bout de 7 jours. Après un tel essai, supprimer `%APPDATA%\com.eauseche.automatpdf\confidentialite.json` : ce fichier est partagé avec l'application installée, qui sinon ne poserait pas la question et reprendrait le numéro de l'essai.

## 11. Petit écran

Pour reproduire l'écran d'un portable : *Paramètres > Système > Écran*, **Échelle** à 150 % (ou 175 %), sur un écran de 1920 × 1080. Remettre la valeur d'origine à la fin.

1. Lancer Automat-PDF.
   **Attendu** : la fenêtre s'ouvre entière ; son bas ne passe pas sous la barre des tâches.
2. Agrandir la fenêtre (bouton carré en haut à droite, ou double-clic sur la barre de titre).
   **Attendu** : le bas de la fenêtre s'arrête au-dessus de la barre des tâches ; le menu de gauche défile jusqu'à sa dernière entrée.
3. **Mot de passe**, un PDF choisi, **Protéger**.
   **Attendu** : le bouton **Protéger le PDF…** reste visible en bas de la fenêtre, détaché par une ombre, tant que le bas de la page n'est pas affiché ; en descendant jusqu'en bas, il reprend sa place sous les réglages. Une fois le PDF enregistré, l'encadré vert vient à l'écran.
4. **Organiser**, un PDF ajouté : double-cliquer sur une page.
   **Attendu** : l'aperçu en grand tient entier dans la fenêtre : page, flèches et légende, sans rien faire défiler.
5. Rétrécir la fenêtre en tirant son coin, le plus possible.
   **Attendu** : elle s'arrête à 900 × 460 environ ; chaque page défile. Dans **Signature ou cachet**, un PDF choisi, la page passe sous les réglages au lieu de déborder.

## 12. Désinstaller

1. *Paramètres > Applications* : désinstaller Automat-PDF.
2. Rouvrir une fenêtre de l'Explorateur et faire un clic droit sur un PDF, une image et un document Word.

**Attendu** : l'entrée Automat-PDF a disparu partout. Réinstaller ensuite l'application pour continuer à s'en servir.

## Résultats à renvoyer

| Partie | Résultat (✅ / ❌) | Remarques, messages, captures |
|---|---|---|
| 1. Compilation et installation | | |
| 2. Clic droit dans l'Explorateur | | |
| 3. Glisser-déposer | | |
| 4. Outils PDF | | |
| 5. Office en PDF | | |
| 6. PDF en Word | | |
| 7. Séparer par personne | | |
| 8. Envoi par e-mail (8a messagerie, 8b Outlook) | | |
| 9. Réglages enregistrés | | |
| 10. Mises à jour et statistiques | | |
| 11. Petit écran | | |
| 12. Désinstallation | | |

Les parties 1, 2, 5, 6 et 8 sont les plus importantes : ce sont celles qui n'ont jamais pu être essayées.

## Premier passage sous Windows (4 novembre 2026)

Sur un PC Windows 11 : 8 Go de mémoire, projet dans OneDrive, Office 2019 dont la licence a expiré, LibreOffice installé, Outlook classique sans compte. Les clics dans l'Explorateur ont été reproduits en lançant les commandes inscrites par l'installateur, les boutons de l'application actionnés par l'accessibilité de Windows ; chaque résultat a été vérifié sur capture d'écran ou sur les fichiers produits.

| Partie | Résultat | Remarques |
|---|---|---|
| 1. Compilation et installation | ✅ après correction | `npm ci` échouait (« Unknown command: "pm" ») : corrigé. Compilation hors de OneDrive. Installateur de 4,1 Mo, installé en 5 s. |
| 2. Clic droit | ✅ | 19 extensions, libellés accentués intacts ; essais 2.1 à 2.5 conformes. Le 2.5 (plusieurs fichiers, application fermée) n'ouvrait que le premier : corrigé. **Reste à voir** : l'aspect du sous-menu dans l'Explorateur. |
| 3. Glisser-déposer | à faire | |
| 4. Outils PDF | ✅ pour le cœur | Les 63 tests du cœur passent sous Windows, essai réel avec Office compris ; l'interface reste à essayer. |
| 5. Office en PDF | ✅ après corrections | Essai 5.1 dans l'application installée, les trois documents ouverts ensemble par le clic droit : « 3 fichiers convertis » en 2 minutes, sans fenêtre d'Office, rien laissé en arrière-plan. Word en PDF par Word ; Excel et PowerPoint, impossibles à piloter sur ce PC, relayés par LibreOffice. Les présentations n'étaient jamais converties, sur aucun PC (le programme transmettait « power-point » au script, qui attend « powerpoint ») : corrigé. Le PowerPoint de l'utilisateur n'est plus refermé, et Office n'est plus laissé caché en arrière-plan, même quand le script s'interrompt. |
| 6. PDF en Word | ✅ après correction | Word attendait une réponse au message « Word va maintenant convertir votre PDF… » : évité. DOCX en 15 à 45 s, sans fenêtre. |
| 7. Séparer par personne | ✅ pour le cœur | L'interface reste à essayer. |
| 8. Envoi par e-mail | partiel | **8a**, par la messagerie : essayé dans l'application, vers un serveur SMTP d'essai installé sur ce PC. Test de la connexion réussi, puis 5 messages reçus, chacun avec sa pièce jointe, identique octet pour octet au fichier créé, accents intacts ; serveur arrêté : « Impossible de joindre le serveur… », rien n'est parti. **Reste à faire** avec un vrai compte (Gmail, Yahoo, messagerie de l'organisation). **8b**, par Outlook : Outlook fermé, message attendu ; pas de compte Outlook sur ce PC, envoi non essayé. |
| 9. Réglages enregistrés | à faire | |
| 10. Désinstallation | ✅ | Programme, raccourci, inscription et les 19 sous-menus retirés. |

**Essai réel avec Office**, sur un PC Windows où Office ou LibreOffice est installé : `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored` convertit les fichiers d'essai (Word et Excel en PDF, PDF en Word) et vérifie les fichiers produits. Il ne couvre pas PowerPoint, faute de présentation parmi les fichiers d'essai : l'essai 5.1, avec une présentation à vous, s'en charge.

**Installation de LibreOffice depuis l'application** (5 novembre 2026, même PC). LibreOffice 7.6 a été désinstallé, puis **Office en PDF** a affiché « LibreOffice n'est pas installé » avec **Installer LibreOffice**. Après confirmation et l'accord de l'administrateur, LibreOffice 26.8.2 s'est installé en 13 min 30 s, dont environ 5 min de téléchargement. L'avertissement a disparu, et l'essai réel a converti le classeur Excel par LibreOffice 26.8.2. Défaut trouvé et corrigé : lancée sans les droits d'administrateur, l'installation silencieuse échouait sans rien demander (Windows Installer, erreur 1730).

**PDF en Word sans Word** (5 novembre 2026, même PC). Word étant installé ici, l'application s'en sert : le convertisseur d'Automat-PDF a donc été essayé hors de l'application, sur les PDF d'essai et sur ceux qu'Office tire des documents d'essai. Les documents obtenus, rouverts par Word et par LibreOffice, ont été comparés page à page aux PDF : convocation identique, tableau compris ; bulletins de paie très proches, filets et en-têtes compris, chaque ligne du récapitulatif de la page 9 sur une seule ligne ; colonnes de la liste des agents alignées ; page numérisée reprise en image. Reste à faire l'essai 6.3 dans l'application, sur un PC sans Word.

**Prise en main sans formation** (6 novembre 2026, même PC). L'application, lancée en mode développement, a été pilotée par les outils de développement de WebView2 ; les fichiers sont arrivés comme par le clic droit, et les fenêtres d'enregistrement de Windows ont été validées par l'accessibilité. Conformes : l'accueil, la recherche (« je veux réduire la taille » ne donne que **Compresser**), un PDF déposé sur l'accueil (13 outils proposés) ; **Compresser** de bout en bout, la fenêtre d'enregistrement ouverte dans le dossier du PDF, le résultat enregistré à côté de l'original et **Afficher dans le dossier** qui ouvre l'Explorateur sur ce fichier, sélectionné, malgré les espaces et l'accent du nom ; essais 7.2 à 7.5, résultats attendus compris, puis **Ouvrir le dossier**. Défaut trouvé et corrigé pendant l'essai : le retour en haut de page à chaque changement d'outil faisait tomber l'interface sous WebView2, où le défilement renvoie une promesse.

**Mises à jour et statistiques** (9 novembre 2026, même PC). L'application, lancée en mode développement et pilotée de la même façon, a d'abord été essayée avec un faux serveur de statistiques et un faux installateur signé. Conformes : la question au premier lancement, sans aucun envoi avant la réponse ; « Non merci », qui ne tire aucun numéro et n'envoie rien ; les statistiques activées ensuite dans « À propos » ; le signal « installation », puis « ouverture » avec le même numéro au lancement suivant ; le bandeau de mise à jour et « Plus tard » ; une signature falsifiée et une ancienne version resservie, refusées sans lancer l'installateur ; la bonne version téléchargée, l'installateur lancé, puis l'application fermée. Ensuite, vers la vraie adresse du site de Sitech, avec des signaux d'essai : après « Oui, j'accepte », le site a accepté le signal « installation » en 2 secondes ; au lancement suivant, le signal « ouverture » est parti sans erreur. Tant qu'aucune version publiée n'a de `latest.json` (la 0.2.0 n'en a pas), la recherche des mises à jour échoue sans rien afficher. Reste à faire avec l'installateur de la 0.2.1 : la partie 10 en entier, dont l'e-mail « nouvelle installation » et le remplacement de la 0.2.0.

**Petits écrans** (11 novembre 2026, même PC, écran 1920 × 1080 à 125 %). Un utilisateur avait signalé des boutons cachés sous la barre des tâches, fenêtre agrandie. Cause reproduite avec une fenêtre Windows d'essai : quand sa taille minimale dépasse la place utile, Windows l'agrandit à cette taille minimale, plus haute que l'écran (880 pixels pour 816 utiles). Or Automat-PDF exigeait 680 pixels de contenu : sur un écran 1920 × 1080 à 150 %, la fenêtre agrandie débordait de 46 pixels sous la barre des tâches (234 à 175 %, 421 à 200 %) ; elle en exige maintenant 460, et tient dans ces trois cas. Vraie fenêtre de la compilation de développement : elle annonce à Windows un minimum de 900 × 460 ; à l'ouverture, son contenu (1280 × 786) s'arrête au pixel près au-dessus de la barre des tâches, alors qu'avec 820 de haut il passait 43 pixels dessous ; agrandie, de même. L'interface a ensuite été essayée dans Edge sans fenêtre (le moteur de WebView2), cœur simulé, aux tailles de 1280 × 649 (150 %, fenêtre agrandie), 1097 × 546 (175 %), 960 × 469 (200 %) et 900 × 460 : 22 vérifications conformes. Les boutons de la question du premier lancement restent visibles, même quand son texte doit défiler ; **Protéger le PDF…** et **Fusionner…** (14 PDF) restent en bas de la fenêtre, puis reprennent leur place une fois la page descendue ; l'encadré du résultat vient à l'écran ; l'aperçu en grand d'**Organiser** tient entier ; **Signature ou cachet**, **Masquer des passages** et **Numéros et tampon** passent la page sous les réglages sous 1280 de large, sans rien faire déborder ; aucun des 16 écrans ne déborde à droite à 900 de large. Reste à faire sur un portable réglé à 150 % : la partie 11.
