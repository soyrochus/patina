**Prompt for code generator: GitHub Actions to build single-binary “patina” for Windows, macOS, Linux (with embedded assets)**

Goal: Create a CI workflow that produces **one self-contained binary per OS** named **`patina`** (not `patina-app`) and embeds app resources (e.g., logo) into the executable so no external files are required.

### Deliverables

1. `.github/workflows/release.yml` — GitHub Actions workflow that:

* Triggers on `push` tags matching `v*` and on manual dispatch.
* Builds **release** binaries for a matrix of OS runners:

  * `ubuntu-latest` → target `x86_64-unknown-linux-musl` (static, single file)
  * `macos-latest` (native target on runner, typically `aarch64-apple-darwin`)
  * `windows-latest` → target `x86_64-pc-windows-msvc`
* Caches Cargo.
* Installs required Rust targets (musl for Linux).
* Builds with `cargo build --release` (and `--target` where needed).
* Strips symbols where appropriate.
* Renames outputs to **`patina`** (Linux/macOS) and **`patina.exe`** (Windows).
* Uploads artifacts named:

  * `patina-linux-x86_64`
  * `patina-macos-arm64` (or host arch)
  * `patina-windows-x86_64.exe`

2. **Binary name fix**:

* Ensure `Cargo.toml` defines the binary explicitly to avoid `patina-app`:

  ```toml
  [package]
  name = "patina"
  # …

  [[bin]]
  name = "patina"
  path = "src/main.rs"
  ```

3. **Embedded assets** (logo, etc.) — single-file executable:

* Add `assets/logo.svg` (and any other assets).
* Use compile-time embedding (no runtime files). Implement with `rust-embed` (or `include_bytes!` if you prefer zero deps):

  * Add dependency:

    ```toml
    [dependencies]
    rust-embed = { version = "8", features = ["include-exclude"] }
    ```
  * Minimal accessor:

    ```rust
    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "assets/"]
    struct Assets;

    pub fn load_logo_svg() -> &'static [u8] {
        Assets::get("logo.svg").expect("embedded logo").data.as_ref()
    }
    ```
* The app must load UI images from the embedded bytes, not from disk.

4. **Workflow specifics to implement**

* Linux: install musl target and strip:

  ```yaml
  - run: rustup target add x86_64-unknown-linux-musl
  - run: sudo apt-get update && sudo apt-get install -y musl-tools
  - run: cargo build --release --target x86_64-unknown-linux-musl
  - run: strip target/x86_64-unknown-linux-musl/release/patina
  ```
* macOS: build native and strip:

  ```yaml
  - run: cargo build --release
  - run: strip -x target/release/patina
  ```
* Windows: build native and strip (use `llvm-strip` from LLVM tools component):

  ```yaml
  - run: rustup component add llvm-tools-preview
  - run: cargo build --release
  - shell: pwsh
    run: |
      $llvm = (rustup which --toolchain stable llvm-objcopy) -replace "llvm-objcopy","llvm-strip.exe"
      & $llvm target\release\patina.exe
  ```
* Rename and upload artifacts:

  ```yaml
  - name: Rename artifact
    run: |
      # Linux/macOS adjust path accordingly
      mv target/x86_64-unknown-linux-musl/release/patina patina
  - uses: actions/upload-artifact@v4
    with:
      name: patina-linux-x86_64
      path: patina
  ```

5. **Signing/Notarization**
   Do **not** add signing/notarization. Produce raw binaries only.

6. **README note**
   Add a short section stating binaries are single-file, carry embedded assets, and are produced by the GitHub Actions workflow on tag releases.

---

**Output expected from the agent**

* A complete `release.yml` workflow implementing the above matrix, caching, builds, stripping, renaming, and artifact upload.
* Adjusted `Cargo.toml` with explicit `[[bin]] name = "patina"`.
* Minimal embedding code and `assets/` folder scaffold.
* Any small build README update reflecting how to trigger a release (`git tag vX.Y.Z && git push --tags`).
