@echo off
rem Runs the daemon from a copy of the built binary, so `cargo build` and
rem `cargo test` can relink target\debug\shadows.exe while the daemon runs:
rem Windows locks a running executable. Restart the daemon to pick up a new build.
setlocal
set "BIN=%~dp0..\target\debug"
copy /Y "%BIN%\shadows.exe" "%BIN%\shadows-run.exe" >nul
if errorlevel 1 (
  echo run-daemon: could not copy shadows.exe to shadows-run.exe; is another daemon running? 1>&2
  exit /b 1
)
"%BIN%\shadows-run.exe" %*
