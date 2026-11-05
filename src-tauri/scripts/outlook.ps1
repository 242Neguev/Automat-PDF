# Prepare dans les Brouillons d'Outlook (version classique), ou envoie, des messages avec pieces jointes.
# -Donnees : fichier JSON (UTF-8) { "envoyer": bool, "messages": [ { "a", "objet", "texte", "pieces": [] } ] }.
# Compte rendu sur la sortie, une ligne JSON par message : { rang, ok, details } ;
# une ligne { erreur } arrete tout : donnees, outlook-ferme, outlook-absent.
param([Parameter(Mandatory = $true)][string]$Donnees)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false
function Ecrire($objet) {
    [Console]::Out.WriteLine(($objet | ConvertTo-Json -Compress))
    [Console]::Out.Flush()
}
try {
    $envoi = [System.IO.File]::ReadAllText($Donnees, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
} catch {
    Ecrire @{ erreur = 'donnees'; details = $_.Exception.Message }
    exit 1
}
if (-not (Get-Process -Name OUTLOOK -ErrorAction SilentlyContinue)) {
    Ecrire @{ erreur = 'outlook-ferme' }
    exit 2
}
try {
    $outlook = New-Object -ComObject Outlook.Application
} catch {
    Ecrire @{ erreur = 'outlook-absent'; details = $_.Exception.Message }
    exit 3
}
$rang = 0
foreach ($message in $envoi.messages) {
    try {
        $mail = $outlook.CreateItem(0)
        $mail.To = [string]$message.a
        $mail.Subject = [string]$message.objet
        $mail.Body = [string]$message.texte
        foreach ($piece in $message.pieces) { [void]$mail.Attachments.Add([string]$piece) }
        if ($envoi.envoyer) { $mail.Send() } else { $mail.Save() }
        Ecrire @{ rang = $rang; ok = $true }
    } catch {
        Ecrire @{ rang = $rang; ok = $false; details = $_.Exception.Message }
    }
    $rang++
}
exit 0
