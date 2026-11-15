{
  description = "OpenWGT: an open engine for Maxis' Widget Workshop, with its own graphics, sounds and data";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # What the window, the GPU and the sound load at run time (winit,
      # wgpu and cpal open them themselves, so they must be on the path).
      runtimeLibs =
        pkgs: with pkgs; [
          wayland
          libxkbcommon
          vulkan-loader
          libGL
          alsa-lib
          libx11
          libxcursor
          libxi
          libxrandr
        ];
    in
    {
      packages = forAll (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "openwgt";
          version = "0.1.0";
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "-p"
            "openwgt"
          ];
          # The tests that need the CD skip themselves without it.
          cargoTestFlags = [ "--workspace" ];
          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
          ];
          buildInputs = [ pkgs.alsa-lib ];
          postFixup = ''
            wrapProgram $out/bin/openwgt \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (runtimeLibs pkgs)}
          '';
          meta = {
            description = "An open engine for Widget Workshop, with its own graphics, sounds and data";
            license = pkgs.lib.licenses.agpl3Plus;
            mainProgram = "openwgt";
          };
        };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
            pkg-config
            alsa-lib
            # tools/opengfx and tools/opensfx
            (python3.withPackages (p: [
              p.pillow
              p.numpy
              p.scipy
            ]))
            # the browser build (tools/build-web.sh): the CLI must be the
            # version crates/widget/Cargo.toml pins
            wasm-bindgen-cli_0_2_130
            lld
            # the CD image's files, the comparison VM
            fontconfig
            libarchive
            qemu
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (runtimeLibs pkgs);
          # tools/opengfx finds DejaVu and Liberation with fc-match.
          FONTCONFIG_FILE = pkgs.makeFontsConf {
            fontDirectories = with pkgs; [
              dejavu_fonts
              liberation_ttf
            ];
          };
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });

      formatter = forAll (pkgs: pkgs.nixfmt);
    };
}
