# Third-party components in the portable Rust build

SC Bookmarks code remains PolyForm Noncommercial. That license does not replace
licenses for separately bundled libraries, OCR models, fonts, or reference data.

- ONNX Runtime 1.31.0: Microsoft, MIT. Full license and bundled third-party notices
  are in licenses/ONNX-Runtime-*.
- PP-OCRv3 detection and recognition models from rapidocr-onnxruntime 1.2.3:
  PaddlePaddle/PaddleOCR and RapidOCR authors, Apache-2.0. See the accompanying
  PaddleOCR and RapidOCR licenses. Native preprocessing, horizontal region
  extraction and CTC decoding are Rust adaptations of those interfaces; no Python
  interpreter is bundled. Model files are unmodified.
- ort: Rust ONNX Runtime interface, MIT/Apache-2.0; egui/eframe: MIT/Apache-2.0.
  The generated licenses/RUST-DEPENDENCIES.md inventory and per-crate directories
  include the resolved Rust dependency versions and their supplied notices/fonts.
- Microsoft Visual C++ runtime DLLs are unmodified app-local redistributables,
  governed by Microsoft terms, not the project license. Redistribution list:
  https://learn.microsoft.com/en-us/visualstudio/releases/2026/redistribution
  License directory: https://visualstudio.microsoft.com/license-terms/
- Rust standard library license/copyright notices are included separately.

Offline community route/map reference provenance is retained in the source
route-data files. Coordinates: starmap.space and Star Citizen Wiki API. Aaron
Halo band bounds: CaptSheppard/Cornerstone 3.16.1 survey. Ore names: SCMiner.
All map drawing and star backgrounds are generated; no game art is redistributed.

Cloud Imperium Games and Roberts Space Industries had no involvement in this
project. They do not develop, fund, review, sponsor, or endorse it.

Community container centers, rotation periods and phases: starmap.space API snapshot collected 2026-10-21, retained in route-data/community-containers.json. Rotation convention (hours per revolution, phase in degrees, reference 2020-01-01 UTC) cross-checked against https://github.com/Valalol/Star-Citizen-Navigation/blob/main/backend.py. Coordinate math is independently implemented; no source code or game assets are copied. These positions are estimates, not live-verified transforms.

Refinery bonus/method reference data: UEX (https://uexcorp.space/), public
refineries_yields and refineries_methods API snapshot retrieved 2026-10-24.
Per-record report times remain in route-data/refineries-uex.json. Community data
may differ from live servers; project licensing does not override third-party data rights.
Additional map reference provenance, survey assumptions and illustrative belt widths
are retained in route-data/map-features.json and map-reference-points.json.
