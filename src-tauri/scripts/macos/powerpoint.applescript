-- Microsoft PowerPoint pour Mac : presentations en PDF. Partie de office.applescript.
property nomApplication : "PowerPoint"
property processusApplication : "Microsoft PowerPoint"

on demarrer()
	tell application "Microsoft PowerPoint"
		launch
		get version
	end tell
end demarrer

-- La presentation est retrouvee par son nom : une presentation du meme nom deja ouverte (celle de
-- l'utilisateur) arrete la conversion au lieu d'etre confondue avec elle.
on convertir(sorte, source, cible)
	set nom to my nomDe(source)
	set fichier to POSIX file source
	set destination to POSIX file cible
	tell application "Microsoft PowerPoint"
		if exists presentation nom then error "Une presentation du meme nom (" & nom & ") est ouverte dans PowerPoint : fermez-la, puis recommencez." number 1002
		open fichier
	end tell
	-- PowerPoint peut ouvrir la presentation en differe : on l'attend un peu.
	repeat 100 times
		tell application "Microsoft PowerPoint" to set ouverte to exists presentation nom
		if ouverte then exit repeat
		delay 0.2
	end repeat
	tell application "Microsoft PowerPoint"
		if not (exists presentation nom) then error "PowerPoint n'a pas ouvert cette presentation." number 1001
		set diaporama to presentation nom
		try
			save diaporama in destination as save as PDF
		on error explication number numero
			close diaporama saving no
			error explication number numero
		end try
		close diaporama saving no
	end tell
end convertir

on documentsOuverts()
	tell application "Microsoft PowerPoint" to return count of presentations
end documentsOuverts

on documentsModifies()
	set modifies to 0
	tell application "Microsoft PowerPoint"
		repeat with i from 1 to count of presentations
			if not (saved of presentation i) then set modifies to modifies + 1
		end repeat
	end tell
	return modifies
end documentsModifies

on quitter()
	tell application "Microsoft PowerPoint" to quit saving no
end quitter
