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
3. À la fin, le dossier de l'installateur s'ouvre : `src-tauri\target\release\bundle\nsis\`, ou `%LOCALAPPDATA%\Automat-PDF-compilation\target\release\bundle\nsis\` si le projet est dans OneDrive. Lancer `Automat-PDF_0.1.0_x64-setup.exe`.
   - « Windows a protégé votre ordinateur » : cliquer sur **Informations complémentaires**, puis sur **Exécuter quand même**.
   - L'installation se fait pour l'utilisateur courant, sans droits d'administrateur.

**Attendu** : l'application démarre. Le menu compte 15 outils en 7 groupes, de **Organiser** à **PDF en Word**, et défile si la fenêtre est petite. Le bouton ⓘ, en haut à droite, affiche Eau sèche et ses coordonnées.

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

## 4. Outils PDF, en bref

Ils ont tous été essayés sur Mac ; il s'agit de vérifier qu'ils se comportent de même sous Windows, avec des chemins accentués.

1. **Compresser** `Scan courrier.pdf` en **Forte**.
   **Attendu** : « 1,8 Mo → 175 Ko » environ (−90 % au moins) ; la page compressée reste lisible.
2. **Images en PDF** avec `Photo portrait.jpg` : **Créer le PDF…**.
   **Attendu** : une page en portrait, la photo droite.
3. **Signature ou cachet** : `Bulletins octobre.pdf`, puis l'image `Cachet.png`, placée sur la dernière page.
   **Attendu** : le cachet rond, sans carré blanc autour.
4. **Mot de passe** : protéger `Bulletins octobre.pdf`, puis ouvrir le PDF protégé dans Edge.
   **Attendu** : Edge demande le mot de passe et ouvre le fichier avec.
5. **Organiser** : tourner une page, en déplacer une autre, puis **Enregistrer…**. Ouvrir le résultat.

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
2. **Séparer par personne** : **Choisir le PDF…** `Bulletins octobre.pdf`, puis **Importer un fichier Excel ou CSV…** `personnel-essai.csv`.
   **Attendu** : la vérification indique « 6 fichiers · 7 pages attribuées » et 2 pages à vérifier : la page 7 (sans identifiant reconnu) et la page 9 (plusieurs personnes possibles : c'est le récapitulatif).
3. Activer **Une page où personne n'est reconnu suit la précédente**.
   **Attendu** : « 8 pages attribuées » ; seule la page 9 reste à vérifier.
4. Activer **Protéger chaque fichier par un mot de passe propre à la personne**. Dans le tableau de la liste, ligne **Mot de passe**, choisir la colonne **Date de naissance**.
   **Attendu** : la colonne « Mot de passe » de la vérification indique « défini » partout.
5. **Créer les fichiers…** dans le dossier d'essai.
   **Attendu** : un dossier `Bulletins octobre - par personne` avec 6 PDF et `_Pages à vérifier.pdf`. `Bulletin DIALLO Aïssatou.pdf`, ouvert dans Edge, demande un mot de passe : `03/11/1987` l'ouvre, sur une page. `Bulletin OKEMBA Grâce.pdf` (`08/09/1990`) a deux pages.

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

## 9. Enchaînements

1. Toujours dans **Séparer par personne** : **Enregistrer les réglages…**, nom « Bulletins de paie », **Enregistrer**.
2. Fermer l'application et la rouvrir. **Séparer par personne** → **Enchaînement** : choisir « Bulletins de paie ».
   **Attendu** : les réglages reviennent, et la liste `personnel-essai.csv` est rouverte d'elle-même. Après **Choisir le PDF…** `Bulletins octobre.pdf`, les mots de passe sont « définis » sans rien rechoisir.
3. **Supprimer**, puis confirmer.
   **Attendu** : la liste des enchaînements est vide.

## 10. Désinstaller

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
| 9. Enchaînements | | |
| 10. Désinstallation | | |

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
| 9. Enchaînements | à faire | |
| 10. Désinstallation | ✅ | Programme, raccourci, inscription et les 19 sous-menus retirés. |

**Essai réel avec Office**, sur un PC Windows où Office ou LibreOffice est installé : `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored` convertit les fichiers d'essai (Word et Excel en PDF, PDF en Word) et vérifie les fichiers produits. Il ne couvre pas PowerPoint, faute de présentation parmi les fichiers d'essai : l'essai 5.1, avec une présentation à vous, s'en charge.

**Installation de LibreOffice depuis l'application** (5 novembre 2026, même PC). LibreOffice 7.6 a été désinstallé, puis **Office en PDF** a affiché « LibreOffice n'est pas installé » avec **Installer LibreOffice**. Après confirmation et l'accord de l'administrateur, LibreOffice 26.8.2 s'est installé en 13 min 30 s, dont environ 5 min de téléchargement. L'avertissement a disparu, et l'essai réel a converti le classeur Excel par LibreOffice 26.8.2. Défaut trouvé et corrigé : lancée sans les droits d'administrateur, l'installation silencieuse échouait sans rien demander (Windows Installer, erreur 1730).

**PDF en Word sans Word** (5 novembre 2026, même PC). Word étant installé ici, l'application s'en sert : le convertisseur d'Automat-PDF a donc été essayé hors de l'application, sur les PDF d'essai et sur ceux qu'Office tire des documents d'essai. Les documents obtenus, rouverts par Word et par LibreOffice, ont été comparés page à page aux PDF : convocation identique, tableau compris ; bulletins de paie très proches, filets et en-têtes compris, chaque ligne du récapitulatif de la page 9 sur une seule ligne ; colonnes de la liste des agents alignées ; page numérisée reprise en image. Reste à faire l'essai 6.3 dans l'application, sur un PC sans Word.
