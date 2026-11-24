@echo off
rem Cargo runs the runner with cwd = the package dir, so the Python script is
rem found relative to this .cmd, not the cwd. The compiled runner is used when
rem `just install-test-runner` has installed it; the Python script stays the
rem fallback for CI and fresh clones. No parenthesized block: `exit /b` inside
rem one returned 0 for a failing test. The last line, on its own, keeps the
rem child's exit code (cargo sees the test binary's code, not the shim's).
set "UTW_RUNNER=%USERPROFILE%\.cargo\bin\utw-test-runner.exe"
if defined CARGO_HOME set "UTW_RUNNER=%CARGO_HOME%\bin\utw-test-runner.exe"
if not exist "%UTW_RUNNER%" set "UTW_RUNNER="
if defined UTW_RUNNER "%UTW_RUNNER%" %*
if not defined UTW_RUNNER python -S -I "%~dp0test_runner.py" %*
exit /b %ERRORLEVEL%
