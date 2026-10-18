@echo off
cd /d "%~dp0"
if not exist ".venv\Scripts\python.exe" (
 echo Run Setup.cmd first.
 pause
 exit /b 1
)
".venv\Scripts\python.exe" scbm_protocol.py --register
if errorlevel 1 exit /b 1
echo SCBM links registered for this Windows user.
pause
