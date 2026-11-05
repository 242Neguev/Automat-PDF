# Convertit des documents avec Microsoft Office, pilote sans l'afficher : Word, Excel et PowerPoint
# en PDF, ou PDF en document Word modifiable (par Word).
# -Donnees : fichier JSON (UTF-8) { "conversions": [ { "source", "cible", "sorte" } ] },
#   sorte : word, excel, powerpoint ou pdf-word.
# Compte rendu sur la sortie, une ligne JSON par evenement :
#   { demarrage, avant } : lancement de Word, Excel ou PowerPoint ; avant : ses processus deja
#     ouverts (ceux qui apparaissent ensuite sont les siens) ;
#   { pret } : l'application repond ;
#   { rang, ok, absent, details } : une conversion ; absent : application d'Office manquante, ou
#     installee mais impossible a piloter.
# Apres un abandon (application qui ne repondait plus), { "arreter": [ { "application", "avant" } ] }
# arrete ses processus restes bloques : apparus depuis `avant` et sans fenetre.
param([Parameter(Mandatory = $true)][string]$Donnees)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false
function Ecrire($objet) {
    [Console]::Out.WriteLine(($objet | ConvertTo-Json -Compress))
    [Console]::Out.Flush()
}
$travail = [System.IO.File]::ReadAllText($Donnees, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
$processus = @{ 'Word' = 'WINWORD'; 'Excel' = 'EXCEL'; 'PowerPoint' = 'POWERPNT' }

# Processus de l'application apparus depuis `avant` et sans fenetre : les siens, jamais ceux de
# l'utilisateur. Un processus qui se termine pendant qu'on l'examine (une application lancee en
# vain se referme seule) leverait une erreur : il est simplement passe.
function ArreterSiens([string]$nom, $avant) {
    if (-not $processus.ContainsKey($nom)) { return }
    foreach ($p in @(Get-Process -Name $processus[$nom] -ErrorAction SilentlyContinue)) {
        try {
            if ($avant -notcontains $p.Id -and $p.MainWindowHandle -eq 0) { $p.Kill() }
        } catch { }
    }
}

if ($null -ne $travail.arreter) {
    foreach ($lancee in $travail.arreter) { ArreterSiens ([string]$lancee.application) @($lancee.avant) }
    exit 0
}

$ouvertes = @{}
$absentes = @{}
# Application qui tournait deja : PowerPoint n'a qu'une instance, c'est alors celle de l'utilisateur.
$deja = @{}

# Application d'Office demandee, lancee une seule fois et sans fenetre ; $null si elle manque ou ne
# se laisse pas piloter.
function Application([string]$nom) {
    if ($ouvertes.ContainsKey($nom)) { return $ouvertes[$nom] }
    if ($absentes.ContainsKey($nom)) { return $null }
    $avant = @(Get-Process -Name $processus[$nom] -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
    Ecrire @{ demarrage = $nom; avant = $avant }
    $application = $null
    $version = ''
    try {
        $application = New-Object -ComObject "$nom.Application"
        # Une installation d'Office abimee cree l'objet, mais ne repond a rien.
        $version = [string]$application.Version
    } catch { }
    if (-not $version) {
        if ($null -ne $application) {
            try { [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($application) } catch { }
        }
        ArreterSiens $nom $avant
        $absentes[$nom] = $true
        return $null
    }
    switch ($nom) {
        'Word' { $application.Visible = $false; $application.DisplayAlerts = 0 }
        'Excel' { $application.Visible = $false; $application.DisplayAlerts = $false }
        'PowerPoint' { $application.DisplayAlerts = 1 }
    }
    $deja[$nom] = $avant.Count -gt 0
    $ouvertes[$nom] = $application
    Ecrire @{ pret = $nom }
    return $application
}

# Reglages de Word dans le registre de l'utilisateur (HKCU:\...\Office\16.0\Word), s'il sert.
$registreWord = $null
if (@($travail.conversions | Where-Object { @('word', 'pdf-word') -contains [string]$_.sorte }).Count -gt 0) {
    try {
        $version = (Get-ItemProperty -Path 'Registry::HKEY_CLASSES_ROOT\Word.Application\CurVer').'(default)'
        if ($version -match '\.(\d+)$') { $registreWord = "HKCU:\Software\Microsoft\Office\$($Matches[1]).0\Word" }
    } catch { }
}

# PDF en Word : Word demande d'abord confirmation ("Word va maintenant convertir votre PDF...") dans
# une fenetre invisible, et attend ; DisplayAlerts n'y change rien. L'option qui l'en dispense (la
# case "Ne plus afficher ce message") est posee avant son lancement, puis remise comme elle etait.
# Si le script est arrete en route, elle reste posee : comme si la case avait ete cochee.
$optionsWord = $null
$avertissementAvant = $null
if ($registreWord -and @($travail.conversions | Where-Object { [string]$_.sorte -eq 'pdf-word' }).Count -gt 0) {
    try {
        $chemin = "$registreWord\Options"
        $avertissementAvant = (Get-ItemProperty -Path $chemin -Name DisableConvertPdfWarning -ErrorAction SilentlyContinue).DisableConvertPdfWarning
        if (-not (Test-Path $chemin)) { New-Item -Path $chemin -Force | Out-Null }
        Set-ItemProperty -Path $chemin -Name DisableConvertPdfWarning -Value 1 -Type DWord
        $optionsWord = $chemin
    } catch { }
}

# Chaque document est ouvert dans une copie, a un emplacement neuf (dossier de travail, efface
# ensuite). Un document pendant l'ouverture duquel Office a ete arrete de force (bloque, plante) est
# en effet tenu pour dangereux a cet emplacement : Office demanderait "Voulez-vous toujours
# l'ouvrir ?" dans une fenetre invisible, et attendrait. Les documents de l'utilisateur ne sont
# ainsi jamais marques.
$copies = Join-Path (Split-Path -Parent $Donnees) 'copies'
function Copie([string]$source, [int]$rang) {
    $dossier = Join-Path $copies ([string]$rang)
    New-Item -ItemType Directory -Path $dossier -Force | Out-Null
    $copie = Join-Path $dossier (Split-Path -Leaf $source)
    Copy-Item -LiteralPath $source -Destination $copie -Force
    return $copie
}

$applications = @{ 'word' = 'Word'; 'pdf-word' = 'Word'; 'excel' = 'Excel'; 'powerpoint' = 'PowerPoint' }
try {
    $rang = 0
    foreach ($conversion in $travail.conversions) {
        $sorte = [string]$conversion.sorte
        $nom = $applications[$sorte]
        $application = Application $nom
        if ($null -eq $application) {
            Ecrire @{ rang = $rang; ok = $false; absent = $nom }
        } else {
            try {
                $source = Copie ([string]$conversion.source) $rang
                $cible = [string]$conversion.cible
                # Mot de passe factice : un document protege echoue au lieu d'ouvrir une fenetre.
                switch ($sorte) {
                    'word' {
                        $document = $application.Documents.Open($source, $false, $true, $false, 'x')
                        try { $document.ExportAsFixedFormat($cible, 17) } finally { $document.Close(0) }
                    }
                    'pdf-word' {
                        $document = $application.Documents.Open($source, $false, $true, $false, 'x')
                        try { $document.SaveAs2($cible, 16) } finally { $document.Close(0) }
                    }
                    'excel' {
                        $classeur = $application.Workbooks.Open($source, 0, $true, 5, 'x')
                        try { $classeur.ExportAsFixedFormat(0, $cible) } finally { $classeur.Close($false) }
                    }
                    'powerpoint' {
                        $presentation = $application.Presentations.Open($source, -1, 0, 0)
                        try { $presentation.SaveAs($cible, 32) } finally { $presentation.Close() }
                    }
                }
                Ecrire @{ rang = $rang; ok = $true }
            } catch {
                Ecrire @{ rang = $rang; ok = $false; details = $_.Exception.Message }
            }
        }
        $rang++
    }
    foreach ($nom in @($ouvertes.Keys)) {
        $application = $ouvertes[$nom]
        $restants = 0
        try {
            $restants = switch ($nom) {
                'Word' { $application.Documents.Count }
                'Excel' { $application.Workbooks.Count }
                'PowerPoint' { $application.Presentations.Count }
            }
        } catch { }
        # Ni le PowerPoint de l'utilisateur, ni une application ou des documents restent ouverts.
        $garder = ($nom -eq 'PowerPoint' -and $deja[$nom]) -or $restants -gt 0
        if (-not $garder) {
            try { $application.Quit() } catch { }
        }
        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($application)
    }
} finally {
    if ($optionsWord) {
        if ($null -eq $avertissementAvant) {
            Remove-ItemProperty -Path $optionsWord -Name DisableConvertPdfWarning -ErrorAction SilentlyContinue
        } else {
            Set-ItemProperty -Path $optionsWord -Name DisableConvertPdfWarning -Value $avertissementAvant -Type DWord -ErrorAction SilentlyContinue
        }
    }
}
exit 0
