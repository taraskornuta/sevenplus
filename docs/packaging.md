# Building installers

The [release workflow](../.github/workflows/release.yml) builds every package automatically when a `v*` tag is pushed. This page covers building them by hand.

All commands run from the repository root. The Rust version is pinned in `rust-toolchain.toml`, and `rustup` installs it automatically the first time you run `cargo`.

## Before you start (all platforms)

Generate the third-party license list that Help ▸ Licenses shows. A copy is already committed, so you only need this after changing dependencies:

```sh
cargo install cargo-about --features cli --locked
cargo about generate -o assets/third-party-licenses.txt about.hbs
```

To regenerate the icons after editing `assets/icons/sevenplus.svg` (needs ImageMagick and Pillow):

```sh
python3 packaging/scripts/make_icons.py
```

## Plain executable

```sh
cargo build --release
```

The binary is `target/release/sevenplus` (`sevenplus.exe` on Windows). It is self-contained: fonts, themes and translations are compiled in.

## Linux: .deb and .rpm

Install the tools once:

```sh
cargo install cargo-deb cargo-generate-rpm --locked
# Debian/Ubuntu build host:
sudo apt install libxkbcommon-dev libwayland-dev dpkg-dev lintian rpm rpmlint
```

Build and package:

```sh
cargo build --release -p calc-app
cargo deb -p calc-app --no-build                 # → target/debian/sevenplus_<version>-1_<arch>.deb
cargo generate-rpm -p crates/calc-app            # → target/generate-rpm/sevenplus-<version>-1.<arch>.rpm
```

Check the packages:

```sh
lintian --fail-on error target/debian/*.deb
rpmlint target/generate-rpm/*.rpm
```

Notes:

- Build the `.deb` on Debian or Ubuntu. `cargo deb` uses `dpkg-shlibdeps` to work out library dependencies (`$auto`); on other distributions it builds the package without them.
- **Old distributions** (PKG-008): a binary links against the glibc of the machine it was built on. To run on every supported distribution, down to openSUSE Leap 15.5, build against glibc 2.31 with [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild), then package for that target:

  ```sh
  cargo install cargo-zigbuild --locked          # also needs `zig` on PATH
  cargo zigbuild --release -p calc-app --target x86_64-unknown-linux-gnu.2.31
  cargo deb -p calc-app --no-build --target x86_64-unknown-linux-gnu
  cargo generate-rpm -p crates/calc-app --target x86_64-unknown-linux-gnu
  ```

  For ARM64, use `aarch64-unknown-linux-gnu` and build on an ARM64 machine, so that `dpkg-shlibdeps` can inspect the binary.
- To sign the RPM: `rpm --define "_gpg_name <key name>" --addsign target/generate-rpm/*.rpm`.

## Windows: .msi and portable .zip

Run these on Windows. You need the [WiX Toolset 3](https://github.com/wixtoolset/wix3/releases) on `PATH` (for example `choco install wixtoolset --version=3.14.1`) and cargo-wix:

```powershell
cargo install cargo-wix --locked
cargo build --release -p calc-app
cargo wix -p calc-app --no-build --nocapture --output target\packages\ -L -sice:ICE57
```

This produces `target\packages\sevenplus-<version>-x86_64.msi`. The installer asks whether to install just for you (no admin rights needed) or for all users. It adds a Start-menu shortcut and an uninstall entry.

For ARM64, add the target and pass it to both commands:

```powershell
rustup target add aarch64-pc-windows-msvc
cargo build --release -p calc-app --target aarch64-pc-windows-msvc
cargo wix -p calc-app --no-build --nocapture --target aarch64-pc-windows-msvc --output target\packages\ -L -sice:ICE57
```

To make the portable zip:

```powershell
Compress-Archive -Path target\release\sevenplus.exe, LICENSE -DestinationPath target\packages\sevenplus-windows-x64-portable.zip
```

To sign the executable and the installer (PKG-002), use `signtool` from the Windows SDK. Sign the `.exe` before running `cargo wix`, then sign the `.msi`:

```powershell
signtool sign /f cert.pfx /p <password> /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 target\release\sevenplus.exe
signtool sign /f cert.pfx /p <password> /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 target\packages\*.msi
```

The executable's icon, version information and DPI-aware manifest (`packaging/windows/sevenplus.exe.manifest`) are added by `crates/calc-app/build.rs` when building on Windows.

## macOS: .app and .dmg

Run these on macOS:

```sh
cargo install cargo-packager --locked
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# Universal binary for Apple silicon and Intel
cargo build --release -p calc-app --target aarch64-apple-darwin
cargo build --release -p calc-app --target x86_64-apple-darwin
mkdir -p target/release
lipo -create -output target/release/sevenplus \
  target/aarch64-apple-darwin/release/sevenplus \
  target/x86_64-apple-darwin/release/sevenplus

cargo packager -p calc-app --release --formats app,dmg   # → target/packages/*.dmg
```

To sign and notarize, set these environment variables before running `cargo packager`, then staple the ticket to the `.dmg`:

```sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: …"
export APPLE_ID=… APPLE_PASSWORD=… APPLE_TEAM_ID=…        # app-specific password
cargo packager -p calc-app --release --formats app,dmg
xcrun stapler staple target/packages/*.dmg
```

`packaging/macos/Info.plist` is merged into the bundle. It enables high-resolution rendering and dark mode and sets macOS 12 as the minimum version.

## Releasing

1. Bump `version` in the root `Cargo.toml`.
2. Add an entry to `packaging/linux/changelog` and a `<release>` to `packaging/linux/io.github.taraskornuta.sevenplus.metainfo.xml`.
3. Commit, then tag and push:

   ```sh
   git tag v0.2.0 && git push origin v0.2.0
   ```

The workflow then runs one hour of fuzzing, builds every package listed above, signs them if the signing secrets are configured, and attaches them to a GitHub release with a `SHA256SUMS` file.
