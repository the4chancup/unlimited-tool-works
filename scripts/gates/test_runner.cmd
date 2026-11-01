@echo off
rem Cargo runs the runner with cwd = the package dir, so the Python script is
rem found relative to this .cmd, not the cwd. `exit /b` keeps the child's
rem exit code (cargo sees the test binary's code, not the shim's).
python "%~dp0test_runner.py" %*
exit /b %ERRORLEVEL%
