{
  description = "OpenWGT: an open engine for Maxis' Widget Workshop, with its own graphics, sounds and data";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  # The package built in two steps, the dependencies on their own: a
  # change to OpenWGT's own code builds only its four crates again.
  inputs.crane.url = "github:ipetkov/crane";

  outputs =
    {
      self,
      nixpkgs,
      crane,
    }:
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

      # The package and its checks, from what the build reads: the
      # workspace's crates (the packs inside them) and its lints.
      builds = forAll (
        pkgs:
        let
          craneLib = crane.mkLib pkgs;
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./clippy.toml
              ./rustfmt.toml
              ./crates
            ];
          };
          common = {
            inherit src;
            inherit (craneLib.crateNameFromCargoToml { cargoToml = ./crates/widget/Cargo.toml; }) pname version;
            strictDeps = true;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.alsa-lib ];
          };
          # Every crate the workspace depends on, built once: cached until
          # Cargo.lock changes.
          cargoArtifacts = craneLib.buildDepsOnly common;
        in
        {
          package = craneLib.buildPackage (
            common
            // {
              inherit cargoArtifacts;
              cargoExtraArgs = "--locked -p openwgt";
              # (the tests are a check of their own)
              doCheck = false;
              nativeBuildInputs = common.nativeBuildInputs ++ [ pkgs.makeWrapper ];
              postFixup = ''
                wrapProgram $out/bin/openwgt \
                  --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (runtimeLibs pkgs)}
              '';
              meta = {
                description = "An open engine for Widget Workshop, with its own graphics, sounds and data";
                homepage = "https://gustavn.se/openwgt";
                license = pkgs.lib.licenses.agpl3Plus;
                mainProgram = "openwgt";
              };
            }
          );
          # The tests that need the CD skip themselves without it.
          tests = craneLib.cargoTest (
            common
            // {
              inherit cargoArtifacts;
              cargoTestExtraArgs = "--workspace";
            }
          );
          clippy = craneLib.cargoClippy (
            common
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--workspace --all-targets -- -D warnings";
            }
          );
        }
      );
    in
    {
      packages = forAll (pkgs: {
        default = builds.${pkgs.stdenv.hostPlatform.system}.package;
      });

      # nix flake check: the build, the tests and clippy.
      checks = forAll (pkgs: {
        inherit (builds.${pkgs.stdenv.hostPlatform.system}) package tests clippy;
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
