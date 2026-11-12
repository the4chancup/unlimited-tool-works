@echo off
setlocal
cd /d "%~dp0"
if "%~1"=="" goto nodrop
set "savefile=%~f1"
set "found=0"
for /d %%D in ("%~dp0old_exports\*") do set "found=1"
for %%F in ("%~dp0old_exports\*.zip" "%~dp0old_exports\*.7z") do set "found=1"
echo.
if "%found%"=="0" goto noexports
if exist "%~dp0upgrade_log.txt" del "%~dp0upgrade_log.txt"
"%~dp0UTW.exe" --log-file "%~dp0upgrade_log.txt" export-upgrader upgrade --all "%~dp0old_exports" --savefile "%savefile%" --legacy-ids -o "%~dp0exports"
set "exit_code=%errorlevel%"
echo Info messages are left out above. The full report is in upgrade_log.txt.
pause
if "%exit_code%"=="0" (exit /b 0) else (exit /b 1)

:nodrop
echo To upgrade the exports in old_exports, drag your PES savefile onto quick_upgrade.bat.
echo The savefile is the file named EDIT00000000, in the game's save folder under Documents\KONAMI.
echo Nothing was upgraded.
pause
exit /b 2

:noexports
echo There is no export in old_exports. Put the old exports there (folders, .zip or .7z files), then drag the savefile onto quick_upgrade.bat again.
pause
exit /b 1
