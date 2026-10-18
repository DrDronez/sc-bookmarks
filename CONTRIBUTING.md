# Contributing

Report reproducible bugs with the release version, Windows/Python versions, display setup, and a redacted error. Do not post personal bookmarks, unredacted captures, private chat, or credentials.

Keep the application observe-and-overlay only. Changes must not send input to the game, access its process memory, inject code, inspect game network traffic, or read/write its files. Use synthetic fixtures in tests and retain the independent-project notice.

Install requirements in a virtual environment and run `python -m unittest discover -s tests -v` and `python -m compileall -q .` before proposing changes. Contributions must be your own work or appropriately licensed; identify third-party material. Project contributions are offered under the repository's PolyForm Noncommercial license. Any separate commercial licensing of contributed work must respect its contributors' rights.
