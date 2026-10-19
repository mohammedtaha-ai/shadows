@echo off
set "NYLRA_BACKEND_URL=https://127.0.0.1:8443"
set "NYLRA_DEV_CA=%~dp0..\shadows-backend\.local-runtime\tls\ca.pem"
set "AYLUL_HOOK_EXE=%~dp0apps\desktop\hook\target\debug\aylul-hook.exe"
start "" "%~dp0apps\desktop\src-tauri\target\debug\nylra-desktop.exe"
