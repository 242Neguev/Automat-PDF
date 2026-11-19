-- Microsoft Word pour Mac : documents en PDF, et PDF en document Word modifiable (Word convertit
-- le PDF a son ouverture). Partie de office.applescript.
property nomApplication : "Word"
property processusApplication : "Microsoft Word"

on demarrer()
	tell application "Microsoft Word"
		launch
		get version
	end tell
end demarrer

-- Word rend le document qu'il a ouvert : rien n'est fait d'un document de l'utilisateur.
on convertir(sorte, source, cible)
	tell application "Microsoft Word"
		if sorte is "pdf-word" then
			set leDocument to open file name source confirm conversions false add to recent files false
		else
			-- Mot de passe factice : un document protege echoue au lieu d'ouvrir une fenetre.
			set leDocument to open file name source password document "x" read only true add to recent files false
		end if
		try
			if sorte is "pdf-word" then
				save as leDocument file name cible file format format document default
			else
				save as leDocument file name cible file format format PDF
			end if
		on error explication number numero
			close leDocument saving no
			error explication number numero
		end try
		-- Enregistre en document Word, il porte desormais le nom (unique) du fichier cree.
		if sorte is "pdf-word" then set leDocument to document (my nomDe(cible))
		close leDocument saving no
	end tell
end convertir

on documentsOuverts()
	tell application "Microsoft Word" to return count of documents
end documentsOuverts

on documentsModifies()
	set modifies to 0
	tell application "Microsoft Word"
		repeat with i from 1 to count of documents
			if not (saved of document i) then set modifies to modifies + 1
		end repeat
	end tell
	return modifies
end documentsModifies

on quitter()
	tell application "Microsoft Word" to quit saving no
end quitter
