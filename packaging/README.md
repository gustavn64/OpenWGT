# Packaging

What the CI builds on every push to master (`.github/workflows/build.yml`)
and publishes on a release (a tag `v*`), and what the package managers
around it take. None of it carries the CD's data.

| Where | What | How |
|---|---|---|
| The release | `openwgt-x86_64.AppImage`, `openwgt-aarch64.AppImage` | CI (linuxdeploy) |
| The release | `openwgt-x86_64.deb`, `openwgt-aarch64.deb`, `.rpm`s | CI (cargo-deb, cargo-generate-rpm; `crates/widget/Cargo.toml`) |
| The release | `openwgt-linux-x86_64.tar.gz`, `-aarch64` | CI: the bare program |
| The release | `openwgt-windows-x86_64.exe`, `openwgt-windows-arm64.exe` | CI |
| The release | `openwgt-macos.dmg` (Apple silicon and Intel) | CI |
| The release | `openwgt-web.zip`: the site, the front page with the game in `play/` | CI (`docs/web.md`) |
| ghcr.io | `ghcr.io/gustavn64/openwgt` (`:latest`, `:X.Y.Z`, `:X.Y`): the site served by nginx, linked to this repository | CI (`docker/Dockerfile`) |
| Nix | `nix run github:gustavn64/OpenWGT` | `flake.nix` |
| AUR | `openwgt`, built from the release's source | `aur/PKGBUILD` |
| winget | `gustavn64.OpenWGT` | `winget/*.yaml` |
| Homebrew | a cask in a tap of one's own (`gustavn64/homebrew-tap`) | `homebrew/openwgt.rb` |
| Flathub | `se.gustavn.openwgt` | `flatpak/` |

## A release

1. Set the version in the four `crates/*/Cargo.toml` (and `Cargo.lock`),
   commit, tag `vX.Y.Z` and push the tag. The CI builds everything, makes
   the release with every file on it and pushes the container image.
2. The checksums the package managers want, from the release's files:
   `sha256sum` of the source tarball
   (`https://github.com/gustavn64/OpenWGT/archive/refs/tags/vX.Y.Z.tar.gz`),
   the two `.exe` and the `.dmg`.
3. **AUR:** `pkgver` and `sha256sums` in `aur/PKGBUILD`, then
   `makepkg --printsrcinfo > .SRCINFO` and push both to
   `ssh://aur@aur.archlinux.org/openwgt.git`.
4. **winget:** the version and the two checksums in `winget/*.yaml`, then a
   pull request to `microsoft/winget-pkgs` with the three files under
   `manifests/g/gustavn64/OpenWGT/X.Y.Z/` (`wingetcreate` can do it).
5. **Homebrew:** `version` and `sha256` in `homebrew/openwgt.rb`, copied to
   `Casks/openwgt.rb` of the tap repository `gustavn64/homebrew-tap`.
6. **Flathub:** the tag in `flatpak/se.gustavn.openwgt.yml`, a release in
   the metainfo, and `cargo-sources.json` made anew from `Cargo.lock`
   (`flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json`, from
   `flatpak/flatpak-builder-tools`); the first time a submission to
   Flathub, afterwards its own repository `flathub/se.gustavn.openwgt`.
   Flathub wants screenshots in the metainfo (`<screenshots>`, pictures
   at a stable address) before the first submission; it has none yet.
