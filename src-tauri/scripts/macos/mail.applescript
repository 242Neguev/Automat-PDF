-- Prepare dans les Brouillons de Mail, ou envoie, des messages avec pieces jointes, comme
-- scripts/outlook.ps1 avec Outlook sous Windows. Automat-PDF place devant ce script la partie
-- commune (commun.applescript).
-- Donnees : { "envoyer": bool, "messages": [ { "a": [adresses], "objet", "texte", "pieces": [] } ] }.
-- Compte rendu, une ligne JSON par message : { rang, ok, details } ; une ligne { erreur } arrete
-- tout : donnees, refus (macOS ne permet pas a Automat-PDF de piloter Mail), mail-absent,
-- sans-compte (aucun compte de messagerie dans Mail).

on run argv
	try
		set envoi to my lireDonnees(item 1 of argv)
	on error explication
		my ecrire({erreur:"donnees", details:explication})
		return
	end try
	-- Sans compte, Mail ouvre son assistant (ajouter un compte) et ne repond plus : deux minutes
	-- suffisent aussi pour accepter la demande de macOS (Automat-PDF veut piloter Mail).
	try
		with timeout of 120 seconds
			tell application "Mail" to set comptes to count of accounts
		end timeout
	on error explication number numero
		if numero is -1743 then
			my ecrire({erreur:"refus", details:explication})
		else
			my ecrire({erreur:"mail-absent", details:explication})
		end if
		return
	end try
	if comptes is 0 then
		my ecrire({erreur:"sans-compte"})
		return
	end if
	set envoyer to |envoyer| of envoi
	set rang to 0
	repeat with courrier in (|messages| of envoi)
		try
			with timeout of 240 seconds
				my preparer(courrier, envoyer)
			end timeout
			my ecrire({rang:rang, ok:true})
		on error explication number numero
			if numero is -1712 then error explication number numero
			my ecrire({rang:rang, ok:false, details:explication})
		end try
		set rang to rang + 1
	end repeat
end run

-- Un message : envoye, ou range dans les Brouillons.
on preparer(courrier, envoyer)
	set pieces to {}
	repeat with chemin in (|pieces| of courrier)
		set end of pieces to POSIX file (chemin as text)
	end repeat
	tell application "Mail"
		set nouveau to make new outgoing message with properties {subject:(|objet| of courrier as text), content:(|texte| of courrier as text) & linefeed & linefeed, visible:false}
		tell nouveau
			repeat with adresse in (|a| of courrier)
				make new to recipient at end of to recipients with properties {address:(adresse as text)}
			end repeat
			repeat with piece in pieces
				tell content to make new attachment with properties {file name:(contents of piece)} at after the last paragraph
			end repeat
		end tell
		-- Mail joint les fichiers en arriere-plan : envoye aussitot, le message partirait sans eux.
		delay 1
		if envoyer then
			send nouveau
		else
			close nouveau saving yes
		end if
	end tell
end preparer
