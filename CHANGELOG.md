# Changelog



## 0.0.5-alpha — 2026-10-24

- Recover coordinate row locations independently when CamDir is readable but fixed coordinate crops fail.
- Locate labeled SolarSystem/Root rows and text bounds, require agreement, cache the successful layout, and reset it on capture-size changes.
- Keep discovery bounded and rate-limited; reuse discovered camera bounds when fixed camera crops fail.
- Keep coordinate failure diagnostics visible between heading-only samples.

Offline replay recovered the same position from a supplied 2560x1440 game region, shifted coordinate rows with readable CamDir, and 1.5x text scaling. The original screenshot also reads with the old fixed path offline, so this does not claim to reproduce or resolve every live capture failure. Additional machine testing is needed.

## 0.0.4-alpha — 2026-10-24

- Publish the Rust application and portable Windows x64 build with bundled OCR models/runtime; no Python installation required for the portable app.
- Retain compatible bookmarks, mining fields, notes, sharing, categories, scanning, QT routes/alerts, map and overlays, with explicit import from previous data folders.
- Improve bounded OCR scheduling, cross-row validation and guidance continuity; use 15 Hz arrow animation and alternating system/shard checks.
- Enlarge the map with a fixed three-row guidance section that does not resize on reading changes.
- Add configurable global F10 overlay toggle and Classic/MISC/Anvil/Drake/Aegis/Argo visual themes; remove the strafe reminder.
- Add the off-by-default 3D bookmark projection test with manual vertical FOV. Live alignment remains experimental.
- Keep Python 0.0.3-alpha source as a reference; preserve independent-project notices and PolyForm Noncommercial licensing.

Known limits: OCR can miss text; projected marker FOV/alignment needs live testing; exclusive fullscreen and other physical display configurations are not guaranteed.

## 0.0.3-alpha — 2026-10-23



- Add a map interface with orbit, pan, zoom, system/route framing, and route profile.

- Select bookmarks and inspect clickable QT-point labels on the map.

- Add a cached starfield, approximate star positions, and clearer route markers.

- Add primary-planet rings for Stanton and Pyro, plus ten textured Aaron Halo

  bands with visibility switches, band selection, and zoom-dependent labels.

- Retain bookmark library, scan references, capture settings, and flight guidance.



Nyx primary-planet rings are unavailable in the current catalog. Halo geometry

uses an approximate Cornerstone 3.16.1 survey; texture represents no actual rocks.

Map views and cached rendering were smoke-tested; all 31 automated tests pass.

Tested game build remains 4.10.2-LIVE.12913470.





## 0.0.2-alpha — 2026-10-23



- Show suggested QT marker pairs beside bookmarks, without adding overlay clutter.

- Show a single planet or moon when a bookmark falls within its cataloged ROI.

- Bundle 70 community destinations across Stanton, Pyro, and Nyx. Compare finite

  routes within the same system, account for arrival radii, and reject modeled

  paths through cataloged bodies.

- Cache suggestions when sorting or filtering bookmarks. No additional OCR work.



The interface shows marker names only. Routes are offline estimates: some station

positions use container centers, ROI uses container radii, and live QT availability

is not checked. Catalog snapshot: 2026-10-21, Wiki build 4.10.2-LIVE.12881860.

Tested game build remains 4.10.2-LIVE.12913470. All 31 automated tests pass.



## 0.0.1-alpha — 2026-10-18



Initial public source prerelease: screen-capture/OCR bookmarks, manual and asteroid entry, mining metadata and filters, sharing via scbm URIs, navigation overlays and QT alerts, scanning reference tables and RS overlays, saved overlay presets, and independent-project documentation.



Tested game build: Star Citizen 4.10.2-LIVE.12913470.



Known limitations: OCR depends on visible text; HUD glare and overlap can cause missed or incorrect readings. Guidance and QT timing are estimates. Broad hardware, resolution, DPI, and game-version compatibility remain unverified.



Local experiments do not increment the published version. Update version.py and this changelog when preparing the next intentional public release.

