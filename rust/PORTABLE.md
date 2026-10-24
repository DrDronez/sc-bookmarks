# SC Bookmarks 0.0.10-alpha — portable Rust build

Extract all ZIP contents into a writable folder, then open **SC-Bookmarks.exe**.
No Python, pip, Rust compiler, model download, or administrator access is required.
Keep the runtime and licenses folders beside the executable. Windows 10/11 x64,
Windows Graphics Capture support, and an OpenGL-capable graphics driver are required.

In Star Citizen enable `r_displayinfo 3`. SC Bookmarks starts capture automatically when the game window is available. The game
may be in the background; its window must be restored and continue rendering.
Capture is read-only. This program does not send game keystrokes, inject code,
read game memory, modify game files, or access game network traffic.

Select a bookmark to show its route and flight overlay. In QT the overlay shows
closest-approach timing; arm the audible exit cue and exit QT manually. Out of QT,
CamDir supplies left/right/up/down guidance and ETA at current closing speed.
Keep the cockpit view centered with freelook off. A stale reading pauses steering
and alerts while retaining an explicitly marked distance estimate.

Use Bookmark here or Capture in 8s for a ship location. Bookmark asteroid reads
the visible SCAN RESULTS, centered target range, coordinates and orientation and
opens a review editor before saving. Review OCR values. Mining permits repeated
ore rows and inert material; derived SCU updates from size and composition.

The library supports category order/rename (Mining stays fixed), notes, filtering
by ore and minimum quality, sorting, copy/paste links, and confirmed deletion.
Double-click a bookmark to edit. The Scanning tab contains all three reference
charts. Identify RS contacts is enabled by default; turn it off on the Capture tab if desired.
Full searches are throttled; located badges receive smaller follow-up reads.
Labels last up to ten seconds and identify the RS value they describe.

Settings places the flight overlay anywhere within the game window and saves
named ship presets. The map includes Stanton, Pyro and Nyx, QT markers, selected
routes, your location, approximate stars, planet guides and Aaron Halo bands.
Route profile magnifies offsets independently; it is not a second distance scale.

Use **Import previous data folder** and select the previous `navigator-data`
folder by entering its path. Import is available before saving a new library;
otherwise copy/paste bookmark links to merge. Original data is never changed.
Portable data is stored in `navigator-data-rust` beside this executable. Back up
that folder. If it is not writable, move the application to your user folders.

**Enable scbm links** registers this copy for `scbm://` links for the current
Windows user. Shared links load into the running instance. Re-enable after moving
the application. Registration does not require administrator rights.

This is a fully independent community project. Cloud Imperium Games, Roberts
Space Industries and their affiliates had no involvement in its development,
funding, review or distribution and do not sponsor or endorse it. Star Citizen
and related names remain their owners' trademarks. No game artwork is bundled.
The game reference build was 4.10.2-live.12913470; OCR can vary with game updates,
lighting, UI scaling and Windows capture support. Community map data is approximate.

Project code: PolyForm Noncommercial (see LICENSE). Separate dependencies and
runtime/model assets retain their own licenses; see THIRD_PARTY_NOTICES.md and
licenses/. Release downloads: https://github.com/DrDronez/sc-bookmarks/releases.


## Overlay controls

F10 hides/shows all overlay visuals, without stopping capture or disarming audible alerts. Settings lets you choose another F-key and optional modifiers; Windows hotkey conflicts are reported in the app status. Save to apply. This uses a Windows hotkey registration, never sends game keystrokes, and does not hook the game.

Settings offers Classic, MISC, Anvil, Drake, Aegis and Argo-inspired visual themes. These are original color/line treatments, not official artwork. Classic remains default. The “Don't strafe” reminder has been removed; guidance uses camera orientation and tolerates lateral motion.

The optional **3D bookmark test** defaults off and adds a projected diamond within 5 km in the current system outside QT. Set the camera's **vertical FOV** in Settings (initially 60 degrees); changing zoom may require adjustment. Alignment has not yet been verified across cockpit cameras. This indicates a saved estimated point, not an object's silhouette or occlusion. Normal guidance remains available for offscreen targets. The marker hides on stale readings.

Desktop overlays are intended for borderless/windowed play. Exclusive-fullscreen behavior is not guaranteed. No universal OCR accuracy or FPS claim is made.

## Community bookmarks and capture controls

The Community tab contains 2,901 researched POIs, kept separate from your personal library. Search, sort, or filter by system, type, verified QT status and navigability. Placeholder/uninitialized entries and entries within 1 km in comparable coordinate frames have been removed. Data is a snapshot and may be outdated. Entries with supported coordinates can be navigated to or copied without capture. Planet-local coordinates retain their container and use a clock-based rotation estimate. Entries missing coordinates or using unsupported frames explain why actions are unavailable.

Capture starts automatically when the game window becomes available. Capture-dependent buttons are disabled until usable. Settings offers **Hide yellow capture border (if Windows allows)**, applied on the next capture start; unsupported configurations fall back to normal capture.

Portable ZIP layout: `SC-Bookmarks.exe`, runtime libraries, README and notices are at the archive root; `runtime/` and `licenses/` remain required support folders. Extract everything into your chosen application folder. GitHub-generated Source code archives retain GitHub's standard enclosing directory; use the Windows x64 release asset to run the app.

Arrival ETA uses `hh:mm:ss`. With fresh current coordinates, the interface also shows **Closest approach via [QT marker] (from here)** when a finite jump leg improves the approach by more than 1 km and catalog uncertainty. The catalog route remains visible; the map shows that catalog route. Suggestions are geometric and do not verify game QT availability, spool-up or minimum jump distance. The title bar identifies the running application version.

In Settings, enable **3D bookmark marker (experimental)** and **Also show beyond 5 km (same system only)** for a distant projected diamond. The extension defaults off. It requires a known matching system and fresh position/camera readings; it is hidden in QT, behind the camera and offscreen. Manual FOV alignment remains experimental. Distance labels switch from meters to kilometers at range.

Community destinations now support source system coordinates and supported container-local positions. Container-local destinations use a clock-based rotation estimate, refreshed once per second; validate them in game because source data and clock accuracy affect alignment. Version-3 links preserve the container and local coordinates and require 0.0.10-alpha or newer. Copy and queue navigation without capture. Missing coordinates and unsupported tilted frames cannot provide valid destinations and display a reason. Editing XYZ of a container-relative bookmark converts it to a fixed system-coordinate bookmark.
