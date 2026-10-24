# Rust port validation

Validated locally on Windows x64, October 23–24, 2026.

- All 32 unit/integration tests pass, including the Python reference fixtures
  listed in README.md. Final release build succeeds.
- Portable --verify-runtime passes: both local models load, recognition confidence
  0.9792, exact synthetic coordinate text, six detected regions. Process exit 0.
- Loaded ONNX and VC runtime modules resolve inside the portable application folder.
- Live background WGC read coordinates, CamDir, Stanton and a full shard ID;
  Bookmark here saved UTC metadata and correct coordinates to isolated test data.
- Saved Nyx game frame: exact XYZ, CamDir, system and shard. Native worker replay
  reaches eight accepted samples, fresh navigation, successful bookmark creation
  and AT BOOKMARK guidance. Routine recognition latency was 0.64–0.82 seconds in
  that replay; the initial metadata pass was 1.91 seconds. No pending frame queue.
- Saved asteroid fixture: mass 20747, resistance 0, instability 15.09, SCU 30.71,
  Iron 12.48%/521 and 41.73%/325, Inert 45.77%/0, range 99.1m.
- Saved 8,000 RS fixture: Surface 2 ROC deposits / Space 4 salvage scrap pieces.
- Native overlay creation/hide test passes; own windows are non-activating,
  click-through. Game geometry is queried read-only.
- GUI launched from portable folder, map and library inspected, live capture,
  selected destination, route profile, stored bookmark restart and URI handoff
  exercised. Original Python data is not changed by these tests.
- Removed detector arena retention; recognition keeps its arena for latency.
  Queue sizes, scan frequency, model threads and image dimensions are bounded.
  Working set depends on capture size and active scans; no universal FPS guarantee.

Physical checks used this PC. The test game subsequently entered a loading screen
without coordinates; the final navigation timing check used recorded game pixels.
Other Windows builds, graphics drivers and physical multi-monitor/DPI layouts
remain platform acceptance work, not verified claims. OCR lighting failures remain
possible. QT timing uses geometry/state tests rather than an automated game flight.
No game keystrokes or memory access were used. No GitHub publication was performed.

Overlay correction: replaced WM_PAINT/color-key presentation with a 32-bit DIB and UpdateLayeredWindow using explicit opaque alpha. Removed capture exclusion from our separate overlay windows. Compositor calls succeed, all 32 tests pass, and the user confirmed the panel is visibly displayed in-game with a test bookmark selected. Portable ZIP refreshed; no GitHub publication.

Foreground correction: installed the independently refreshed topmost-window build (the previous compilation had not reached the portable copy). After activating Star Citizen in Borderless, a captured foreground view showed the Iron / Inert Materials asteroid panel with distance and direction. This verifies this session on this PC, not exclusive-fullscreen or all-platform support. RS contacts now default on. Release build and all 32 tests pass.

Floating guidance: navigation now uses a transparent DIB background and outlined centered text with enlarged direction arrows. RS result cards retain their background. Foreground placement remains unchanged. Release build and all 32 tests pass; installed and restarted locally. Visual in-game acceptance remains to be confirmed.
Original marker restoration: ported FlightOverlay clamped/normalized single-arrow geometry, green alignment ring, and original text positions from SCBookmarks.py. Added numeric geometry regression test; all 33 tests pass and release build succeeds. Installed binary hash verified. Transparent overlay confirmed above game menu; live steering visual check unavailable because game was at main menu.
Marker continuity: normal-flight display has a six-second read grace, annotated when estimates/heading are held. Bookmark creation and QT alerts retain 2.5-second freshness. Native small-surface animation interpolates angles with a 0.18-second time constant and shortest-path yaw wrap. OCR cadence unchanged. Added grace and smoothing regressions; all 35 tests pass. Release binary installed and hash verified.
OCR continuity: narrowed navigation rows, strengthened observed tiny midline minus strokes before CamDir recognition (no textual sign guessing), preserved navigation samples during full RS scans and removed early polling return. System/shard reads now take at most one auxiliary row per sample. Live isolated run: 17/17 position and camera reads, 470-753ms. End-to-end 25s worker test consistently fresh after eight-reading warmup, 0.8-1.0s observed result age. All 37 tests pass, including minus-versus-dot morphology and navigation-crop preservation. Installed release hash verified. These timings are one stationary scene on this machine, not universal OCR/FPS claims.
GPS cadence: capture gate 250ms; heading-only OCR between ~900ms coordinate cycles. Partial heading observations do not refresh position age or enter eight-position average (regression test). After initial system/shard identification, 60-second alternating slots give each a 120-second cadence; retry batches bounded to 12 position cycles. Shard bookmark age limit accommodates the new cadence. Overlay interpolation capped at 67ms (~15Hz). All 38 tests pass. Live heading-only result ages 0.1-0.3s; full coordinate passes still occupy the shared OCR worker and can delay the next heading sample. Installed binary verified.
Glare follow-up: tested raw/threshold/local-contrast/inverted views on the same captured pixels; thresholding was worse and was not adopted. Taller 180px CamDir crop, tighter 400px coordinate crops, one alternate crop retry, cross-row coordinate consensus, conservative duplicate-minus/decimal-space normalization, and large heading change confirmation added. Full layout rediscovery avoided when CamDir confirms fixed layout. 41 tests pass. Live 25s pipeline: all accepted positions matched observed XYZ, and freshness remained true after warmup despite individual rejected reads; heading-only ages mostly 0.1-0.2s, normal position ages 0.4-0.7s, retries ~1.0-1.3s. Some OCR misses remain; stationary-scene result is not an all-lighting guarantee. Binary installed and hash verified.

System-map layout: enlarged map area by flattening guidance to three fixed rows, reserving a 124px destination dock and a 30px status bar, and reducing the always-reserved route profile to 72px. Long text truncates with hover details. Release build passed; installed executable hash verified and restarted. Visual check at 1442x972 confirmed identical map bounds with no selection, selected/waiting, and live guidance; bottom controls and status text are unclipped. Capture resumed with existing asteroid bookmark selected. No publication or version bump.

Overlay test features: added off-by-default projected bookmark diamond (same system, normal flight, <=5 km, fresh camera/position), explicit manual vertical FOV control, default F10 global visual toggle via RegisterHotKey/MOD_NOREPEAT with conflict reporting, and saved Classic/MISC/Anvil/Drake/Aegis visual themes. Removed strafe reminder; camera-based guidance already supports lateral motion. No new capture/OCR work or game input. Projection tests cover center, lateral movement, roll, aspect ratios, behind-camera/range/nonfinite rejection; settings tests cover backward defaults and invalid FOV/hotkeys. Previous 41 tests passed; two new tests passed (43 total). Release built and installed with hash verification. Settings visually checked; F10 toggled runtime hidden status both directions, restored enabled. 3D test left off and Classic unchanged. Live marker alignment/zoom and foreground-game hotkey behavior remain user acceptance checks; no in-game keystrokes were sent. No publication or version bump.

Argo theme: added orange primary / warm ivory accents and squared industrial brackets with paired index marks. Uses existing theme selector/persistence for navigation, projected marker and scan cards. Release build passed, installed executable hash verified, app restarted. Existing selected theme preserved; no version bump or publication.

0.0.4-alpha release validation: all 43 tests passed. Fresh Windows x64 package passed --verify-runtime (recognition confidence 0.9792, six detected regions, exit 0). Portable ZIP includes runtime/models, licenses and per-file SHA256 manifest; package contains no navigator-data folders. Source includes the synthetic OCR self-check fixture and Rust Windows CI.
