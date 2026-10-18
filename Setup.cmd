@echo off
cd /d "%~dp0"
py -3.14 -m venv .venv
if errorlevel 1 goto failed
".venv\Scripts\python.exe" -m pip install -r requirements.txt
if errorlevel 1 goto failed
echo Setup complete. Run Start SC Bookmarks.cmd.
pause
exit /b 0
:failed
echo Setup failed. Install 64-bit Python 3.14 with pip and Tcl/Tk, then retry.
pause
exit /b 1
