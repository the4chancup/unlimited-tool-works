@echo off
rem Cargo runs the runner with cwd = the package dir, so the Python script is
rem found relative to this .cmd, not the cwd. `exit /b` keeps the child's
rem exit code (cargo sees the test binary's code, not the shim's).
rem The compiled runner is used when `just install-test-runner` has installed
rem it; the Python script stays the fallback for CI and fresh clones.
set "UTW_RUNNER=%USERPROFILE%\.cargo\bin\utw-test-runner.exe"
if defined CARGO_HOME set "UTW_RUNNER=%CARGO_HOME%\bin\utw-test-runner.exe"
if exist "%UTW_RUNNER%" ("%UTW_RUNNER%" %* & exit /b)
python -S -I "%~dp0test_runner.py" %*
exit /b %ERRORLEVEL%
