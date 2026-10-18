# SC Bookmarks

**0.0.1-alpha — Windows source release.** Save and share locations, record mining scans, and receive on-screen navigation and scan-reference guidance from visible game pixels.

**Tested game build: Star Citizen 4.10.2-LIVE.12913470.** Other builds have not been validated; changes to debug text, HUD layout, or scanning values may affect results.

**Fully independent project:** SC Bookmarks was developed independently by DrDronez. Cloud Imperium Games (CIG), Roberts Space Industries (RSI), and their affiliates had no involvement in this project's development. They do not sponsor, endorse, publish, or support this application. Star Citizen and related names and trademarks belong to their respective owners and are used solely to identify compatibility. No official artwork or logos are included.

## Observe and overlay only

The application captures the rendered Star Citizen window through Windows screen-capture APIs, reads visible text with OCR, and draws its own desktop overlay. You remain in control of the game.

- No keystrokes or mouse actions are sent to the game.
- No game-process memory is read or written, and no code is injected into the game.
- No game files, logs, configuration files, or network packets are read or modified.
- Bookmarks, settings, and diagnostic captures are written only to the application's own data folder.
- Optional `scbm://` registration changes a Windows per-user URL association, not the game.

These are descriptions of the implementation, not claims of approval or guaranteed compatibility with anti-cheat systems. This project does not grant permission under the game's terms. Review the current [RSI terms](https://robertsspaceindustries.com/en/tos). OCR and navigation estimates can be wrong.

## Install

Requirements: **64-bit Windows**, **64-bit Python 3.14 with Tcl/Tk and pip**, and an internet connection for the initial dependency installation. Development testing used Windows 11. Other Python versions and Windows configurations have not been validated. This release is Python source, not a standalone executable. Tesseract is not required.

1. Install Python from [python.org](https://www.python.org/downloads/windows/), including the Python launcher, pip, and Tcl/Tk.
2. Download `sc-bookmarks-0.0.1-alpha.zip` from Releases and extract it into a writable folder. Do not run it inside the ZIP or put it in the game's installation directory.
3. Run **Setup.cmd**. It creates a local `.venv` and installs the packages listed in `requirements.txt` from PyPI.
4. Run **Start SC Bookmarks.cmd**. Setup is only needed once per installation or when dependencies change.

Manual equivalent, from the extracted folder:

```powershell
py -3.14 -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r requirements.txt
.\.venv\Scripts\python.exe SCBookmarks.py
```

## Game setup and first bookmark

**`r_displayinfo 3` is required for bookmarking and location detection.** Open the game's console yourself and enter:

```text
r_displayinfo 3
```

Keep the coordinate/debug text visible and unobstructed. Orientation guidance also needs readable CamDir information. Keep the cockpit camera centered with freelook off when using ship guidance. The app never enters console commands for you.

Start the game and the application, then wait for Root XYZ readings. Use **Bookmark here**, **Bookmark asteroid**, or manual bookmark entry. Single-click selects a bookmark; double-click opens its editor. Review OCR-populated data before saving. Use the navigation action to start guidance. QT alerts are audible cues: **you must exit quantum travel manually**. Outside QT, arrows and estimated arrival time help with the approach.

Windowed and borderless layouts use the game client area's dimensions, with automatic coordinate cropping. The game may be unfocused but must continue rendering. Minimized windows, exclusive fullscreen, unusual scaling, HUD overlap, and mixed-DPI monitors can affect capture or OCR; not every configuration has been tested. Overlay placement uses normalized coordinates, with named presets in Settings.

## Features

- Create, edit, rename, delete, categorize, filter, and share bookmarks; free-form notes.
- UTC creation timestamps displayed in local PC time. Shard IDs provide context without restricting navigation; verify them after server changes.
- Mining scans: mass, resistance, instability, total SCU, repeated ore entries, percentages, quality, estimated SCU, and inert materials. Filter by ore and minimum quality.
- Asteroid capture estimates the target position from ship coordinates, camera direction, and displayed range. This is an approximation, not an exact game entity position.
- QT closest-approach cue and normal-flight direction, distance, and ETA.
- Three offline scanning reference charts and contextual RS labels. Signal signatures may have multiple explanations and can change with game updates.
- Adjustable overlay position and named ship presets.

## Share and open links

Use the copy/import controls to share `scbm://` text. Links can include coordinates, system, shard, creation time, category, notes, and mining data. Review what you share; recipients can read this information.

Run **Enable SCBM Links.cmd** once to associate the URI scheme with this installation for your Windows user. A link pasted into Windows Run or opened by a browser can then load a bookmark into SC Bookmarks. No administrator rights are required. Some chat clients do not make custom URI schemes clickable; copying and importing the text still works. Re-register after moving the installation. Opening a link does not start game capture automatically.

## Local data and troubleshooting

Personal files are stored in `navigator-data/` beside the application. Back up that entire folder before upgrading. Extract a new release separately, copy your data folder into it, run Setup, and re-register links if used. Releases contain no personal bookmarks or captures.

The application has no telemetry or upload feature. Diagnostic captures and logs can contain visible game text, coordinates, shard identifiers, and chat: inspect and redact them before sharing. Dependency installation uses internet access; normal capture and OCR run locally.

If readings stop, check `r_displayinfo 3`, capture status, the correct game window, and text visibility. Avoid placing the overlay over debug text. Stale position estimates are temporary and should not be treated as fresh measurements. If startup fails, run `.venv\Scripts\python.exe SCBookmarks.py` from a terminal to see the error. Include the app version, Windows/Python versions, display resolution/scaling, window mode, and a redacted error in a GitHub issue.

## License and releases

Copyright (c) 2026 DrDronez. Licensed under [PolyForm Noncommercial 1.0.0](LICENSE). Personal and other permitted noncommercial uses are free under its terms. Commercial use requires separate permission from DrDronez; open a licensing inquiry on this repository. This is source-available software, not an unrestricted open-source license. Third-party components retain their own licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Version numbers change for intentional public releases containing significant updates, not for every local test or modification. The initial prerelease is **0.0.1-alpha**. See [CHANGELOG.md](CHANGELOG.md).
