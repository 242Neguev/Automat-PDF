# Fiche de test sous Mac

Ce qui est propre au Mac : la compilation et l'image disque, l'installation, le clic droit dans le Finder (services), « Ouvrir avec », le dépôt sur l'icône du Dock, les conversions par Microsoft Office pour Mac, l'envoi par Mail, la barre des menus et les mises à jour. Le reste de l'application (les outils PDF, la séparation par personne, l'envoi par la messagerie) est le même que sous Windows : la [fiche de test sous Windows](../test-windows/README.md) le décrit en détail, et ses parties 4, 6, 7, 8a, 9 et 11 se suivent telles quelles sur Mac.

**Version à tester** : celle qui contient ce fichier, ou plus récente.

## Avant de commencer

- Un Mac sous macOS 11 (Big Sur) ou plus récent, à processeur Intel ou Apple Silicon, avec Internet.
- Microsoft Office pour Mac (Word, Excel, PowerPoint), **ouvert une fois à la main** : ses fenêtres de premier lancement (bienvenue, confidentialité, connexion) bloqueraient sinon son pilotage (parties 5 et 6). Un Mac sans Office est utile aussi : LibreOffice y prend le relais, et Automat-PDF y convertit lui-même les PDF en Word.
- Un compte de messagerie réglé dans l'app **Mail** (partie 7).
- Créer un dossier d'essai dont le nom contient un espace et un accent, par exemple `Documents/Essai Automat été`, et y copier les [fichiers d'essai](../test-windows/fichiers) de la fiche Windows (fictifs) ; tous les fichiers produits iront dedans. Pour PowerPoint, ajouter une présentation à vous.

**En cas de problème** : faire une capture d'écran (`⌘ ⇧ 4`, puis sélectionner la zone), recopier le message exact et noter l'action qui l'a provoqué. Puis passer à la suite : les parties sont indépendantes, sauf la 1.

## 1. Compiler et installer

1. Double-cliquer sur `scripts/compiler-mac.command` (voir le [README](../../README.md#obtenir-lapplication-mac)). Accepter l'installation des outils de ligne de commande d'Apple si la fenêtre d'Apple la propose.
2. À la fin, le Finder montre `Automat-PDF_X.Y.Z_universal.dmg`. L'ouvrir, puis glisser **Automat-PDF** sur le dossier **Applications**.
3. Ouvrir Automat-PDF depuis le Launchpad ou le dossier Applications.
   - Avec une image disque téléchargée (publication GitHub), macOS refuse d'abord : **Terminé**, puis **Réglages Système › Confidentialité et sécurité** › **Ouvrir quand même**. Compilée sur ce Mac, l'application s'ouvre directement.

**Attendu** : l'accueil « Que voulez-vous faire ? », comme sous Windows. La barre des menus, en haut de l'écran, est en français : **Automat-PDF** (À propos d'Automat-PDF, Services, Masquer Automat-PDF, Quitter Automat-PDF), **Fichier**, **Édition** (Annuler, Couper, Copier, Coller, Tout sélectionner), **Présentation**, **Fenêtre**, **Aide**. **À propos d'Automat-PDF** montre le nom, la version et « © 2026 Eau sèche ».

À noter aussi : le temps de compilation.

## 2. Clic droit dans le Finder

macOS range les actions des applications sous **Services**, en bas du menu du clic droit (directement dans le menu quand il y en a peu). Si elles n'y sont pas juste après l'installation, fermer puis rouvrir la session, ou cocher les services « Automat-PDF : … » dans **Réglages Système › Clavier › Raccourcis clavier… › Services › Fichiers et dossiers**.

1. **Quitter Automat-PDF** (`⌘ Q`). Dans le dossier d'essai, clic droit sur `Scan courrier.pdf` → **Services** → **Automat-PDF : Compresser**.
   **Attendu** : les services proposés pour un PDF sont Organiser les pages, Fusionner, Compresser, Protéger par un mot de passe, Séparer par personne, Convertir en Word, Convertir en images. L'application s'ouvre sur **Compresser**, avec « Scan courrier.pdf · 1,8 Mo ».
2. **Sans quitter l'application**, sélectionner `Bulletins octobre.pdf` et `Scan courrier.pdf`, clic droit → **Services** → **Automat-PDF : Fusionner**.
   **Attendu** : la fenêtre déjà ouverte passe au premier plan, sur **Fusionner**, avec les deux fichiers (« 2 fichiers, 10 pages au total »).
3. Clic droit sur `Photo portrait.jpg` → **Services** → **Automat-PDF : Mettre en PDF**.
   **Attendu** : **Images en PDF** montre la photo droite, le mot HAUT en haut.
4. Clic droit sur `Convocation formation.docx` → **Services** → **Automat-PDF : Convertir en PDF**.
   **Attendu** : **Office en PDF** liste le document.
5. Clic droit sur `Bulletins octobre.pdf` → **Ouvrir avec** → **Automat-PDF**.
   **Attendu** : Automat-PDF demande quoi faire du fichier (« Que faire de ce fichier ? »), parmi les 13 outils qui prennent un PDF. Aperçu reste l'application qui ouvre les PDF d'un double-clic.
6. **Quitter Automat-PDF**, puis faire glisser `Scan courrier.pdf` sur l'icône d'Automat-PDF dans le Dock (ou dans le dossier Applications).
   **Attendu** : l'application s'ouvre et propose les outils qui prennent un PDF.

## 3. Glisser-déposer dans la fenêtre

Comme la partie 3 de la fiche Windows, depuis le Finder.

## 4. Outils PDF, en bref

Comme la partie 4 de la fiche Windows. **Ouvrir** affiche le PDF dans Aperçu ; **Afficher dans le dossier** ouvre le Finder sur le fichier, sélectionné. Dans **Organiser**, `⌘ + clic` ajoute une page à la sélection, `⌘ A` sélectionne tout, `⌘ Z` annule.

## 5. Office en PDF

1. **Office en PDF** : **Ajouter des fichiers…** `Convocation formation.docx`, `Liste des agents.xlsx` et une présentation PowerPoint. **Convertir…** dans le dossier d'essai.
   **Attendu** : la première fois, macOS demande si « Automat-PDF » peut contrôler « Microsoft Word » (puis Excel, PowerPoint) : **OK**. Word, Excel et PowerPoint s'ouvrent l'un après l'autre, sans aucune autre question, puis se referment s'ils n'étaient pas ouverts avant ; « 3 fichiers convertis ». Les trois PDF ont la même mise en page que les documents d'origine.
2. Recommencer **avec Word déjà ouvert** sur un autre document.
   **Attendu** : ce document reste ouvert et intact, Word aussi ; les nouveaux PDF s'appellent « Convocation formation (2).pdf »… sans écraser les premiers.
3. Ouvrir `Liste des agents.xlsx` dans Excel, puis le convertir à nouveau.
   **Attendu** : « Un classeur du même nom (Liste des agents.xlsx) est ouvert dans Excel : fermez-le, puis recommencez. » Rien n'est fait du classeur ouvert.
4. Si vous avez un document Word protégé par un mot de passe, l'ajouter et convertir.
   **Attendu** : il est indiqué « non converti », avec la raison, sans bloquer les autres.
5. Refuser l'autorisation (**Réglages Système › Confidentialité et sécurité › Automatisation › Automat-PDF** : décocher Microsoft Word), puis convertir un document Word.
   **Attendu** : « macOS ne permet pas à Automat-PDF de piloter Microsoft Word… », avec le chemin du réglage ; LibreOffice prend le relais s'il est installé. Recocher ensuite Microsoft Word.

**Pourquoi Office travaille dans son propre dossier** : dans son bac à sable, Office pour Mac ne lit et n'écrit sans rien demander que dans son conteneur (`~/Library/Containers/com.microsoft.Word/Data/tmp`…) ; ailleurs, il demanderait pour chaque fichier où l'enregistrer. Automat-PDF y dépose une copie de chaque document, puis range le PDF dans le dossier choisi et efface la copie.

**Si Office ne répond pas pendant 4 minutes** (une fenêtre attend une réponse : bienvenue, connexion, activation) : l'application l'arrête s'il n'était pas déjà ouvert, et convertit le reste sans lui. Ouvrir alors Word, Excel et PowerPoint une fois à la main, fermer leurs fenêtres, puis réessayer.

**Sur un Mac sans Office ni LibreOffice** : la page indique « Ni Microsoft Office ni LibreOffice ne sont installés », avec **Page de téléchargement** et **Vérifier à nouveau**. Télécharger LibreOffice pour macOS, ouvrir le `.dmg`, glisser LibreOffice dans Applications, puis **Vérifier à nouveau** : l'avertissement disparaît.

## 6. PDF en Word

1. **PDF en Word** : `Bulletins octobre.pdf`, puis **Convertir…** dans le dossier d'essai.
   **Attendu** : Word s'ouvre, convertit le PDF (quelques secondes), puis se referme s'il n'était pas déjà ouvert ; `Bulletins octobre.docx` s'ouvre dans Word avec du texte modifiable.
2. **Sur un Mac sans Word** : la page prévient qu'Automat-PDF convertira lui-même (partie 6.3 de la fiche Windows).

## 7. Envoi par Mail

Avec un compte de messagerie réglé dans l'app Mail. Suivre la partie 7 de la fiche Windows (séparer `Bulletins octobre.pdf` par personne, avec `personnel-essai.csv` complété de **votre** adresse), puis :

1. Activer **Envoyer chaque fichier par e-mail**, **Envoyer avec** : **Mail**, garder **Brouillons à vérifier**.
   **Attendu** : « Les messages partent de l'adresse réglée dans l'app Mail… ».
2. **Adresse pour un essai** : votre adresse, puis **Préparer un brouillon d'essai**.
   **Attendu** : la première fois, macOS demande si « Automat-PDF » peut contrôler « Mail » : **OK**. Dans les Brouillons de Mail, un message pour vous, avec `Bulletin NKOULOU MBARGA Jean-Baptiste.pdf` en pièce jointe et le texte « Bonjour Jean-Baptiste, … » sur plusieurs lignes, accents intacts.
3. **Préparer 5 brouillons dans Mail**.
   **Attendu** : 5 brouillons, chacun avec le bulletin de la bonne personne.
4. Choisir **Envoi direct**, puis **Envoyer 5 e-mails…** et confirmer (« depuis Mail »).
   **Attendu** : les 5 messages arrivent dans votre boîte de réception.

L'envoi par **Ma messagerie** (SMTP) se fait comme dans la partie 8a de la fiche Windows ; le mot de passe est gardé dans le trousseau de macOS (app **Trousseaux d'accès**, élément « messagerie.Automat-PDF »).

## 8. Mises à jour

Avec une image disque publiée par GitHub (étiquette `vX.Y.Z`), quand une version plus récente est publiée :

1. Lancer l'ancienne version.
   **Attendu** : le bandeau « Une nouvelle version d'Automat-PDF est disponible ».
2. **Mettre à jour**.
   **Attendu** : barre de téléchargement, puis Automat-PDF se ferme et se rouvre de lui-même, avec la nouvelle version dans **À propos**. macOS peut demander le mot de passe de l'ordinateur pour remplacer l'application dans le dossier Applications.
3. Après la mise à jour, convertir un document Word (partie 5) et envoyer par **Ma messagerie** (partie 7), comme avant.
   **Attendu** : ni la question « Automat-PDF souhaite contrôler Microsoft Word », ni la demande d'accès au trousseau : les deux versions sont signées par le même certificat d'Eau sèche (Terminal : `codesign -dv /Applications/Automat-PDF.app` montre `Authority=Eau sèche - Automat-PDF`), et macOS garde leurs autorisations. Si macOS les redemande, le noter : une version a été signée autrement (« ad hoc »).

## 9. Thème sombre

Comme la partie 12 de la fiche Windows, avec *Réglages Système > Apparence* : **Sombre**, puis **Claire** (ou **Auto**, qui change avec l'heure). La barre de titre et le contenu de la fenêtre changent ensemble, aussitôt.

## 10. Désinstaller

Glisser Automat-PDF du dossier Applications à la Corbeille.

**Attendu** : ses services disparaissent du clic droit (au plus tard après une nouvelle session), et « Ouvrir avec » ne le propose plus. Ses réglages restent dans `~/Library/Application Support/com.eauseche.automatpdf/` (à supprimer à la main pour tout effacer).

## Résultats à renvoyer

| Partie | Résultat (✅ / ❌) | Remarques, messages, captures |
|---|---|---|
| 1. Compilation et installation | | |
| 2. Clic droit dans le Finder | | |
| 3. Glisser-déposer | | |
| 4. Outils PDF | | |
| 5. Office en PDF | | |
| 6. PDF en Word | | |
| 7. Envoi par Mail | | |
| 8. Mises à jour | | |
| 9. Thème sombre | | |
| 10. Désinstallation | | |

## Premier passage sur Mac (18 et 19 novembre 2026)

Sur un MacBook Pro Apple Silicon sous macOS 26.0.1, avec Microsoft Office 16.115 (Word, Excel, PowerPoint, Outlook) et sans LibreOffice ; l'app Mail sans aucun compte. Les services ont été déclenchés par `NSPerformService`, le chemin même du clic droit dans le Finder ; l'écran de l'application a été décrit par son utilisateur.

| Partie | Résultat | Remarques |
|---|---|---|
| 1. Compilation et installation | ✅ | `scripts/compiler-mac.command` : 5 min 20 la première fois (Rust et Node.js déjà là), 3 min ensuite ; application universelle de 24 Mo, image disque de 11,75 Mo, signée ad hoc avec le runtime renforcé. Installée en glissant l'application de l'image disque dans Applications. **Outils de ligne de commande d'Apple** : la fenêtre d'Apple ne pouvait pas les installer sur macOS 26.0.1 (le catalogue d'Apple n'en publie plus que pour macOS 26.2 et suivants) ; les paquets officiels d'Apple, signés, ont été déballés dans `~/.local/share/automat-pdf-compilation/CommandLineTools`, dont le script se sert. |
| 2. Clic droit dans le Finder | ✅ | Les 9 services sont inscrits (`pbs -dump`). **Fusionner** sur deux PDF, application ouverte : les deux fichiers dans l'outil. **Compresser**, application fermée : elle s'ouvre sur Compresser, avec le fichier. **Ouvrir avec** (application fermée) : « Que faire de ce fichier ? » et les outils qui prennent un PDF. Défaut trouvé et corrigé : des fichiers arrivés pour un outil pendant que la question « Que faire de ce fichier ? » restait ouverte étaient cachés par elle ; elle se referme désormais. **Reste à voir** : l'aspect du menu du clic droit dans le Finder, et le dépôt sur l'icône du Dock (même chemin qu'« Ouvrir avec »). |
| 3. Glisser-déposer | à faire | |
| 4. Outils PDF | ✅ pour le cœur | Les 116 tests du cœur passent sur Mac ; Clippy, en mode strict, passe pour macOS et pour Windows (vérification croisée depuis le Mac, qui a trouvé une importation inutile sous Windows, corrigée). L'interface reste à essayer outil par outil. |
| 5. Office en PDF | ✅ | Dans l'application installée : un document Word, un classeur Excel et une présentation PowerPoint ajoutés par le service « Convertir en PDF », puis **Convertir…** vers le Bureau : « 3 fichiers convertis », après l'accord demandé par macOS pour piloter chaque application. Word, Excel et PowerPoint refermés ensuite, aucune copie laissée dans leurs conteneurs. Essai réel du cœur (`cargo test -- --ignored`) : Word et Excel en PDF, PDF en Word, en 10 s. Constaté pendant la mise au point : hors de son conteneur, Office pour Mac demande où enregistrer chaque fichier (d'où le dépôt des copies dans son conteneur) ; la fenêtre de premier lancement d'Excel bloque son pilotage tant qu'elle n'est pas fermée à la main. |
| 6. PDF en Word | ✅ pour le cœur | `Bulletins octobre.docx` créé par Word pour Mac (8 s, sans question de Word) ; à essayer dans l'interface. |
| 7. Envoi par Mail | à faire | Aucun compte dans Mail sur ce Mac : son assistant (« ajouter un compte ») bloque son pilotage. Le message d'Automat-PDF a été revu en conséquence : au bout de 2 minutes, il explique qu'il faut régler son adresse dans Mail (ou choisir « Ma messagerie ») et accepter la demande de macOS. **À faire** sur un Mac dont l'app Mail a un compte. |
| 8. Mises à jour | à faire | Avec deux versions publiées par GitHub. |
| 9. Désinstallation | à faire | |
