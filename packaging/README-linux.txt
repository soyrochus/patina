Patina – Linux (x86_64, musl)
=============================

This is an experimental, UNSIGNED build linked against musl for broad portability.

Requirements
-----------
- x86_64 Linux with a reasonably modern kernel

Install & Run
-------------
1) Extract:
     tar -xzf patina-linux-x86_64.tar.gz
2) Ensure the binary is executable (should be):
     chmod +x ./patina
3) Run:
     ./patina --help

Notes
-----
- Built with musl for portability. Prefer rustls-based TLS crates to avoid system OpenSSL issues.

Integrity
---------
A SHA-256 checksum file is attached on the Release page.

License
-------
See LICENSE in this archive (if included) or in the repository.
