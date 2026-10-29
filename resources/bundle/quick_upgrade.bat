@echo off
setlocal
cd /d "%~dp0"
if "%~1"=="" goto nodrop
set "savefile=%~f1"
set "upgraded=0"
set "not_upgraded=0"
set "found=0"
for /d %%D in ("%~dp0old_exports\*") do (
    set "found=1"
    call :upgrade "%%~fD"
)
for %%F in ("%~dp0old_exports\*.zip" "%~dp0old_exports\*.7z") do (
    set "found=1"
    call :upgrade "%%~fF"
)
echo.
if "%found%"=="0" goto noexports
echo Upgraded: %upgraded%. Not upgraded: %not_upgraded%.
pause
if "%not_upgraded%"=="0" (exit /b 0) else (exit /b 1)

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

:upgrade
echo.
echo Upgrading %~nx1
"%~dp0UTW.exe" export-upgrader upgrade "%~1" --savefile "%savefile%" -o "%~dp0exports"
if errorlevel 1 (set /a "not_upgraded+=1") else (set /a "upgraded+=1")
exit /b
