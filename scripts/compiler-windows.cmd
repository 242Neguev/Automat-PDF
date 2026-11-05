@echo off
rem Double-cliquer pour compiler l'installateur Windows (voir compiler-windows.ps1). Ajouter -Msi pour produire aussi un .msi.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0compiler-windows.ps1" %*
pause
