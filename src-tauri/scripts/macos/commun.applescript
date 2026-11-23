-- Commun aux scripts d'Automat-PDF pour macOS, pendants des scripts PowerShell de Windows :
-- Automat-PDF le place en tete de chacun. Les donnees arrivent par un fichier JSON (UTF-8), dont
-- le chemin est le premier argument ; le compte rendu part sur la sortie, une ligne JSON par
-- evenement.
use AppleScript version "2.5"
use scripting additions
use framework "Foundation"

-- Contenu du fichier JSON, en enregistrement AppleScript.
on lireDonnees(chemin)
	set donnees to current application's NSData's dataWithContentsOfFile:chemin
	if donnees is missing value then error "Donnees illisibles : " & chemin number 1
	set {objet, erreur} to current application's NSJSONSerialization's JSONObjectWithData:donnees options:0 |error|:(reference)
	if objet is missing value then error (erreur's localizedDescription() as text) number 2
	return objet as record
end lireDonnees

-- Une ligne JSON sur la sortie, aussitot ecrite.
on ecrire(enregistrement)
	set {donnees, erreur} to current application's NSJSONSerialization's dataWithJSONObject:enregistrement options:0 |error|:(reference)
	set ligne to (current application's NSString's alloc()'s initWithData:donnees encoding:(current application's NSUTF8StringEncoding))'s stringByAppendingString:linefeed
	(current application's NSFileHandle's fileHandleWithStandardOutput())'s writeData:(ligne's dataUsingEncoding:(current application's NSUTF8StringEncoding))
end ecrire

-- Nom d'un fichier d'apres son chemin.
on nomDe(chemin)
	set delimiteurs to AppleScript's text item delimiters
	set AppleScript's text item delimiters to "/"
	set nom to last text item of chemin
	set AppleScript's text item delimiters to delimiteurs
	return nom
end nomDe

-- Identifiants des processus de ce nom (liste vide s'il n'y en a pas).
on processus(nom)
	set texte to do shell script "/usr/bin/pgrep -x " & quoted form of nom & " || true"
	set identifiants to {}
	repeat with ligne in paragraphs of texte
		if (ligne as text) is not "" then set end of identifiants to (ligne as integer)
	end repeat
	return identifiants
end processus
