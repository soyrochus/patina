Patina – Windows (x86_64)
=========================

This is an experimental, UNSIGNED build. No certification, no code signing.

Requirements
-----------
- Windows 10 or 11 (x64)

Install & Run
-------------
1) Unzip this archive.
2) Windows SmartScreen may warn about an unrecognized app (because it is UNSIGNED).
   To proceed:
   - Right-click the EXE → Properties → check “Unblock” → Apply → OK
   - Or in PowerShell:
       Unblock-File .\patina.exe
3) Run:
   .\patina.exe --help

Notes
-----
- SmartScreen prompts are expected due to lack of a signature.
- You may place the EXE anywhere; adding it to PATH is optional.

Integrity
---------
A SHA-256 checksum file is attached on the Release page.

License
-------
See LICENSE in this archive (if included) or in the repository.
