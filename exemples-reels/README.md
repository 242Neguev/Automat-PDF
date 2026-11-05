# Vrais fichiers de test

Ce dossier reçoit les **vrais fichiers** qui servent à tester Automat-PDF. Son contenu est **exclu de Git** (voir `.gitignore`) : rien de ce que tu y déposes n'est envoyé sur GitHub. Seul ce README l'est.

## Ce qui est utile

Tu peux les ranger dans des sous-dossiers, mais ce n'est pas obligatoire.

| Dossier conseillé | Contenu | Pourquoi |
|---|---|---|
| `bulletins/` | Un PDF de bulletins de paie tel que le produit le logiciel de paie, **anonymisé** si possible | Vérifier la séparation d'après le contenu (matricule, nom) et les noms de fichiers |
| `listes/` | La liste du personnel (Excel) **anonymisée**, avec matricules et adresses e-mail | Vérifier les noms de fichiers, les mots de passe par personne et l'envoi |
| `scans/` | Des scans : rectos et versos scannés séparément, pages de travers ou à l'envers, photos de documents | Vérifier l'intercalage, la rotation, les images en PDF et la compression |
| `divers/` | Des PDF variés : protégés par mot de passe, très lourds, issus de Word, d'Excel ou d'un scanner | Vérifier que tout s'ouvre et que rien ne s'abîme |

Indique-moi quand les fichiers sont déposés : je les testerai et je te ferai un compte rendu.
