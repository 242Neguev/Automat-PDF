-- Microsoft Excel pour Mac : classeurs en PDF. Partie de office.applescript.
property nomApplication : "Excel"
property processusApplication : "Microsoft Excel"

on demarrer()
	tell application "Microsoft Excel"
		launch
		get version
	end tell
end demarrer

-- Le classeur est retrouve par son nom : un classeur du meme nom deja ouvert (celui de
-- l'utilisateur) arrete la conversion au lieu d'etre confondu avec lui.
on convertir(sorte, source, cible)
	set nom to my nomDe(source)
	tell application "Microsoft Excel"
		if exists workbook nom then error "Un classeur du meme nom (" & nom & ") est ouvert dans Excel : fermez-le, puis recommencez." number 1002
		-- Mot de passe factice : un classeur protege echoue au lieu d'ouvrir une fenetre ; ni mise
		-- a jour des liens, ni lecture seule conseillee : aucune question.
		open workbook workbook file name source password "x" update links do not update links read only true ignore read only recommended true add to mru false
		if not (exists workbook nom) then error "Excel n'a pas ouvert ce classeur." number 1001
		set classeur to workbook nom
		try
			save workbook as classeur filename cible file format PDF file format
		on error explication number numero
			close classeur saving no
			error explication number numero
		end try
		close classeur saving no
	end tell
end convertir

on documentsOuverts()
	tell application "Microsoft Excel" to return count of workbooks
end documentsOuverts

on documentsModifies()
	set modifies to 0
	tell application "Microsoft Excel"
		repeat with i from 1 to count of workbooks
			if not (saved of workbook i) then set modifies to modifies + 1
		end repeat
	end tell
	return modifies
end documentsModifies

on quitter()
	tell application "Microsoft Excel" to quit saving no
end quitter
