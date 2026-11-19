-- Convertit des documents avec une application de Microsoft Office pour Mac (Word, Excel ou
-- PowerPoint), comme scripts/office.ps1 sous Windows. Automat-PDF assemble le script : la partie
-- commune (commun.applescript), celle de l'application pilotee (word.applescript,
-- excel.applescript ou powerpoint.applescript : proprietes nomApplication et
-- processusApplication, gestionnaires demarrer, convertir, documentsOuverts, documentsModifies et
-- quitter), puis celle-ci. Il ne lui confie que les conversions de cette application, et seulement
-- si elle est installee : AppleScript demanderait sinon ou elle se trouve.
-- Office, dans son bac a sable, n'ecrit sans rien demander que dans son propre dossier (son
-- conteneur) : Automat-PDF y depose les documents a convertir, et y reprend les fichiers crees.
-- Donnees : { "conversions": [ { "source", "cible", "sorte" } ] }, sorte : word, excel,
--   powerpoint ou pdf-word.
-- Compte rendu, une ligne JSON par evenement :
--   { demarrage, avant } : lancement de l'application ; avant : ses processus deja ouverts (ceux
--     qui apparaissent ensuite sont les siens) ;
--   { pret } : l'application repond ;
--   { rang, ok, absent, refus, details } : une conversion ; absent : application impossible a
--     piloter ; refus : macOS ne permet pas a Automat-PDF de la piloter (Reglages Systeme).
-- Une application qui ne repond pas a temps (une fenetre attend une reponse) arrete le script :
-- AppleScript signale l'erreur -1712, qu'Automat-PDF traite comme un blocage (il arrete alors
-- l'application, si elle n'etait pas deja ouverte, et reprend les conversions suivantes).

-- Delai laisse a l'application a chaque etape, en secondes : moins que le chien de garde
-- d'Automat-PDF (5 minutes).
property delai : 240

on run argv
	set travail to my lireDonnees(item 1 of argv)
	set conversions to |conversions| of travail
	set avant to my processus(processusApplication)
	my ecrire({demarrage:nomApplication, avant:avant})
	try
		with timeout of delai seconds
			my demarrer()
			-- Documents ouverts au lancement : ceux de l'utilisateur, ou le document vierge
			-- qu'Office ouvre de lui-meme.
			set auLancement to my documentsOuverts()
		end timeout
	on error explication number numero
		if numero is -1712 then error explication number numero
		repeat with i from 0 to (count of conversions) - 1
			if numero is -1743 then
				my ecrire({rang:i, ok:false, refus:nomApplication})
			else
				my ecrire({rang:i, ok:false, absent:nomApplication, details:explication})
			end if
		end repeat
		return
	end try
	my ecrire({pret:nomApplication})
	set rang to 0
	repeat with conversion in conversions
		try
			with timeout of delai seconds
				my convertir(|sorte| of conversion as text, |source| of conversion as text, |cible| of conversion as text)
			end timeout
			my ecrire({rang:rang, ok:true})
		on error explication number numero
			if numero is -1712 then error explication number numero
			my ecrire({rang:rang, ok:false, details:explication})
		end try
		set rang to rang + 1
	end repeat
	-- L'application lancee ici est refermee, sauf si l'on s'en sert : document ouvert depuis,
	-- ou modifie. Celle de l'utilisateur reste ouverte.
	if (count of avant) is 0 then
		try
			with timeout of 60 seconds
				if my documentsOuverts() is auLancement and my documentsModifies() is 0 then my quitter()
			end timeout
		end try
	end if
end run
