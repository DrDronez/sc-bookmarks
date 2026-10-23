# Changelog

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
