@echo off
setlocal
cd /d "%~dp0"
if exist "%~dp0compile_log.txt" del "%~dp0compile_log.txt"
"%~dp0UTW.exe" --log-file "%~dp0compile_log.txt" team-compiler compile "%~dp0exports"
set "code=%errorlevel%"
echo.
echo Info messages are left out above. The full report is in compile_log.txt.
pause
exit /b %code%
