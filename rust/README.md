# SC Bookmarks Rust port

The portable application implements the Python 0.0.3-alpha workflows in native
Rust. The Python release and its data remain unchanged. Published as 0.0.4-alpha alongside the retained Python reference.
See PORTABLE.md for usage and THIRD_PARTY_NOTICES.md for bundled dependencies.

## Migration checklist

- [x] Compatible bookmark JSON, unknown metadata, notes, UTC/local timestamps,
  shard metadata, manual coordinates, edit/rename/delete, categories and ordering.
- [x] Mining mass/resistance/instability/SCU, repeated ore rows, inert material,
  optional values, derived SCU, same-row ore/quality filtering and library sorting.
- [x] v1/v2 scbm sharing, copy/paste batches, current-user URI registration,
  single-instance inbox, explicit migration with source data left untouched.
- [x] QT finite-segment pair recommendations, obstruction/arrival geometry, ROI
  single-marker routes, CamDir steering, closing ETA, QT detection and audible cue.
- [x] Screen-only Windows Graphics Capture; background window operation, client
  coordinates, bounded asynchronous OCR, eight-sample filtering and stale holding.
- [x] Native CPU ONNX recognition/detection, fixed/adaptive debug row discovery,
  periodic system/shard sampling, variable-position asteroid and RS recognition.
- [x] Same-frame asteroid coordinates/orientation/range and mining review editor.
- [x] Click-through, non-activating overlays; game-relative placement, presets,
  QT/normal flight modes, RS labels with ten-second retention and bounded scanning.
- [x] Three scanning charts and map systems, clickable QT markers, stars, planet
  rings, Aaron Halo bands/dust, ship/target/routes, top/3D/side and route profile.
- [x] Local travel CSV, manual/automatic system choice, delayed capture and logs.
- [x] Portable Windows executable, app-local runtime/models, license inventory,
  checksums and executable runtime self-check.

## Validation and limits

Core tests include Python-derived reference cases: 160 QT routes, 100 guidance
cases, 176 signal readings, 72 library combinations, 96 overlay/search geometries,
120 map projections, and 52 coordinate text fixtures. Storage, corruption,
unknown metadata, mining, link inbox/locking and migration have regression tests.

Native recognition was exercised on saved game screenshots and the live game
while it was in the background. Saved asteroid OCR recovered mass, instability,
SCU, duplicate iron qualities and inert material; the 8,000 RS fixture recovered
the expected two ROC deposits/four salvage scrap pieces. Native overlays were
created and hidden successfully. The portable executable's --verify-runtime
command loads both bundled models and performs recognition and detection without
Python. GUI and package acceptance evidence is recorded in VALIDATION.md.

This is a functional port, not a claim that every game lighting condition or
Windows/monitor configuration has been tested. Mixed-DPI, 4K, ultrawide and small
client placement have geometry coverage; physical validation used this PC.
Horizontal HUD detector regions replace Python/OpenCV rotated boxes. OCR remains
fallible and asteroid results are reviewed before saving. Capture uses a native
worker thread; unexpected capture-library process faults are not isolated in a
Python subprocess. Older Windows compatibility avoids the optional minimum update
interval property; another older Windows PC has not been physically tested.

## Build and package

On Windows x64 install Rust MSVC and Visual Studio C++ build tools. Runtime/model
files are in runtime/. Run cargo test --tests, cargo build --release, then
./package.ps1. The resulting dist/SC-Bookmarks-portable folder is self-contained;
end users do not need these development prerequisites. Package tools never copy
navigator-data directories. Do not overwrite an existing user's data when updating.

Inference uses one CPU thread per model, disables spinning, bounds each queue to
one frame and gates navigation capture copies to about one per 250ms, with full coordinate OCR about every 900ms. Full scans are on-demand
or throttled; cached contact boxes receive smaller follow-up reads. No game input,
injection, memory reads, game-file access, or network telemetry is used.


### Overlay test controls (0.0.4-alpha)

Open **Settings** to select Classic, MISC, Anvil, Drake, Aegis, or Argo. These are independent, manufacturer-inspired palettes and line treatments; no CIG artwork or logos are bundled. Themes change navigation and scan overlay appearance only. Classic remains the default.

**F10** toggles all overlay visuals globally. Settings supports F6–F12 and optional Ctrl/Alt/Shift modifiers, or disabling the hotkey. Save to apply. Windows reports reserved/conflicting combinations in the app status bar; select a different combination if registration fails. The hotkey uses Windows RegisterHotKey, not game input injection or a keyboard hook. It does not stop capture or disarm audible alerts. Hidden state is session-only. The ordinary Overlay checkbox still controls navigation; RS scanning retains its own enable option.

**3D bookmark test** defaults off. Enable and save to show a hollow diamond at the projected saved position within 5 km, in the same system and outside QT. It uses camera basis (including roll), current position and perspective projection, so lateral movement is supported. The test uses a manually adjustable **vertical FOV**, initially 60 degrees, because the fast CamDir OCR crop does not consistently contain FOV. Match the rendered camera FOV; zoom changes require adjustment. The marker supplements the normal arrow, which remains usable for offscreen/behind-camera targets. It disappears on stale inputs, rather than pinning an inaccurate world marker. This is a saved-point estimate, not an asteroid silhouette, depth/occlusion test, or verified landing aid. Live camera/FOV alignment remains to be tested in game.

Visual research references (inspiration only): [official Spectrum cockpit screenshot](https://robertsspaceindustries.com/spectrum/community/SC/forum/1/thread/corsair-missing-speedometer-and-cpld-vtol-landing-) and [official Advanced Flight HUD feedback](https://robertsspaceindustries.com/spectrum/community/SC/forum/50182/thread/4-0-eptu-feedback-advanced-flight-hud). References span game versions; these themes do not claim to reproduce current manufacturer standards exactly.
