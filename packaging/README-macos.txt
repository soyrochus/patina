Patina – macOS (arm64)
======================

This is an experimental, UNSIGNED and NOT NOTARIZED build. No certificates, no signing.

Requirements
-----------
- Apple Silicon (arm64)
- macOS 12.0 Monterey or newer

Install & Run
-------------
1) Unzip this archive.
2) In Finder, right-click the binary and choose “Open”, then confirm the dialog.
   (Terminal alternative if needed)
     xattr -d com.apple.quarantine ./patina
     ./patina --help

Notes
-----
- Gatekeeper may warn you because this build is NOT signed/notarized.
- If you move the binary into a PATH directory (e.g., /usr/local/bin), admin permissions may be required.

Integrity
---------
A SHA-256 checksum file is attached on the Release page.

License
-------
See LICENSE in this archive (if included) or in the repository.
