<#
.SYNOPSIS
  Compile l'installateur Windows d'Automat-PDF sur ce PC.

.DESCRIPTION
  1. Installe si besoin les prérequis, une seule fois : Node.js LTS, Rust, outils C++ de Visual Studio.
  2. Installe les dépendances de l'interface (npm ci).
  3. Compile l'application et produit l'installateur .exe (et le .msi avec -Msi).
  Compatible avec Windows PowerShell 5.1 (Windows 10 et 11).

.PARAMETER Msi
  Produit aussi un installateur .msi, en plus du .exe.

.EXAMPLE
  Double-cliquer sur scripts\compiler-windows.cmd
#>
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '', Justification = "Script interactif : messages en couleur pour l'utilisateur.")]
param([switch]$Msi)

$ErrorActionPreference = 'Stop'
$racine = Split-Path -Parent $PSScriptRoot

# Codes de retour de winget qui ne sont pas des échecs : déjà installé, aucune mise à jour, redémarrage requis.
$codesWingetAcceptes = @(0, -1978335189, -1978335135, -1978334967)

function Write-Etape([string]$texte) {
    Write-Host ''
    Write-Host "==> $texte" -ForegroundColor Cyan
}

function Exit-AvecErreur([string]$texte) {
    Write-Host ''
    Write-Host "ÉCHEC : $texte" -ForegroundColor Red
    exit 1
}

function Sync-CheminSession {
    # Relit le PATH du système pour trouver les outils installés pendant cette session.
    $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')
    $cargo = Join-Path $env:USERPROFILE '.cargo\bin'
    if ((Test-Path $cargo) -and ($env:Path -notlike "*$cargo*")) {
        $env:Path = "$cargo;$env:Path"
    }
}

function Test-Commande([string]$nom) {
    return [bool](Get-Command $nom -ErrorAction SilentlyContinue)
}

function Install-AvecWinget([string]$id, [string]$nom, [string[]]$options = @('--silent')) {
    Write-Host "Installation de $nom (une seule fois)..."
    $arguments = @('install', '--id', $id, '--exact', '--accept-source-agreements', '--accept-package-agreements') + $options
    & winget @arguments
    if ($codesWingetAcceptes -notcontains $LASTEXITCODE) {
        Exit-AvecErreur "l'installation de $nom a échoué (code $LASTEXITCODE)."
    }
    if ($LASTEXITCODE -eq -1978334967) {
        Write-Host "Un redémarrage du PC sera nécessaire pour terminer l'installation de $nom." -ForegroundColor Yellow
    }
    Sync-CheminSession
}

Write-Etape 'Vérification des prérequis'
if (-not (Test-Commande 'winget')) {
    Exit-AvecErreur "winget est introuvable. Installez « App Installer » depuis le Microsoft Store, puis relancez ce script."
}
Sync-CheminSession

# Node.js 20 ou plus
$versionNode = $null
if (Test-Commande 'node') {
    $versionNode = (& node --version) -replace '^v', ''
}
if (-not $versionNode -or [int]($versionNode.Split('.')[0]) -lt 20) {
    Install-AvecWinget -id 'OpenJS.NodeJS.LTS' -nom 'Node.js LTS'
    if (-not (Test-Commande 'npm')) {
        Exit-AvecErreur "Node.js est installé mais introuvable : fermez cette fenêtre et relancez le script."
    }
}
else {
    Write-Host "Node.js $versionNode : OK"
}

# Outils C++ de Visual Studio, nécessaires pour compiler Rust sous Windows. Ils passent avant Rust :
# sinon l'installateur de Rust propose d'installer lui-même Visual Studio Community.
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$outilsCpp = $null
if (Test-Path $vswhere) {
    $outilsCpp = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
}
if (-not $outilsCpp) {
    Install-AvecWinget -id 'Microsoft.VisualStudio.2022.BuildTools' -nom 'outils C++ de Visual Studio (plusieurs Go, 10 à 30 minutes)' `
        -options @('--override', '--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended')
}
else {
    Write-Host 'Outils C++ de Visual Studio : OK'
}

# Rust (chaîne stable, cible MSVC)
if (-not (Test-Commande 'rustup')) {
    Install-AvecWinget -id 'Rustlang.Rustup' -nom 'Rust (rustup)'
    if (-not (Test-Commande 'rustup')) {
        Exit-AvecErreur "Rust est installé mais introuvable : fermez cette fenêtre et relancez le script."
    }
}
& rustup default stable
if ($LASTEXITCODE -ne 0) {
    Exit-AvecErreur 'impossible de préparer la chaîne Rust stable.'
}

Set-Location $racine

# Dans OneDrive, les fichiers intermédiaires de la compilation (plusieurs Go) seraient envoyés dans
# le nuage, et la compilation ralentie ou interrompue : elle se fait alors dans un dossier local.
$cible = $env:CARGO_TARGET_DIR
if (-not $cible) {
    $dansOneDrive = @($env:OneDrive, $env:OneDriveCommercial, $env:OneDriveConsumer) | Where-Object {
        $_ -and ($racine + '\').StartsWith($_.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)
    }
    if ($dansOneDrive) {
        $cible = Join-Path $env:LOCALAPPDATA 'Automat-PDF-compilation\target'
        $env:CARGO_TARGET_DIR = $cible
        Write-Host "Le projet est dans OneDrive : la compilation se fait dans $cible"
    }
    else {
        $cible = Join-Path $racine 'src-tauri\target'
    }
}

Write-Etape "Installation des dépendances de l'interface (npm ci)"
# npm.cmd, et non npm : appelé par « & », le npm.ps1 de npm 10 retire un seul caractère de la
# commande (celui de « & ») au lieu de « npm », et répond « Unknown command: "pm" ».
& npm.cmd ci --no-fund --no-audit
if ($LASTEXITCODE -ne 0) {
    Exit-AvecErreur 'npm ci a échoué : voir les messages ci-dessus.'
}

$formats = 'nsis'
if ($Msi) {
    $formats = 'nsis,msi'
}
Write-Etape "Compilation de l'application (la première fois : 10 à 20 minutes)"
& npm.cmd run tauri build -- --bundles $formats
if ($LASTEXITCODE -ne 0) {
    Exit-AvecErreur 'la compilation a échoué : voir les messages ci-dessus.'
}

$dossierBundles = Join-Path $cible 'release\bundle'
$installateurs = @(Get-ChildItem -Path $dossierBundles -Recurse -Include '*.exe', '*.msi' -ErrorAction SilentlyContinue |
    Where-Object { $_.Directory.Name -in @('nsis', 'msi') })
if ($installateurs.Count -eq 0) {
    Exit-AvecErreur "aucun installateur trouvé dans $dossierBundles."
}

Write-Etape 'Terminé : installateurs produits'
foreach ($installateur in $installateurs) {
    Write-Host ('  ' + $installateur.FullName) -ForegroundColor Green
}
Write-Host ''
Write-Host "L'application n'est pas encore signée : Windows peut afficher « Windows a protégé votre ordinateur »."
Write-Host "Cliquez alors sur « Informations complémentaires », puis sur « Exécuter quand même »."
Invoke-Item $installateurs[0].DirectoryName
