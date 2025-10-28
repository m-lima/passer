{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
    flake-utils.url = "github:numtide/flake-utils";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
    helper.url = "github:m-lima/nix-template";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      treefmt-nix,
      helper,
      ...
    }@inputs:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        bindgen = pkgs.buildWasmBindgenCli rec {
          src = pkgs.fetchCrate {
            pname = "wasm-bindgen-cli";
            version = "0.2.104";
            hash = "sha256-9kW+a7IreBcZ3dlUdsXjTKnclVW1C1TocYfY8gUgewE=";
          };

          cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
            inherit src;
            inherit (src) pname version;
            hash = "sha256-V0AV5jkve37a5B/UvJ9B3kwOW72vWblST8Zxs8oDctE=";
          };
        };
        server =
          (helper.lib.rust.helper inputs system ./server {
            allowFilesets = [ ./server/res ];
          }).outputs;
        wasmDev =
          (helper.lib.rust.helper inputs system ./web/wasm {
            binary = false;
            mega = false;
            toolchains = fenixPkgs: [
              fenixPkgs.stable.toolchain
              fenixPkgs.targets.wasm32-unknown-unknown.stable.rust-std
            ];
            nativeBuildInputs = pkgs: [ bindgen ];
          }).outputs;
        wasm = helper.lib.rust.helper inputs system ./web/wasm {
          enableRust190Fix = false;
          binary = false;
          mega = false;
          toolchains = fenixPkgs: [
            (fenixPkgs.stable.withComponents [
              "cargo"
              "clippy"
              "rustfmt"
            ])
            fenixPkgs.targets.wasm32-unknown-unknown.stable.rust-std
          ];
          nativeBuildInputs = pkgs: [ bindgen ];
          overrides = {
            commonArgs = {
              doCheck = false;
              CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
            };
          };
        };

        prefixCheck =
          prefix: check:
          pkgs.lib.mapAttrs' (key: value: {
            inherit value;
            name = "${prefix}_${key}";
          }) (builtins.removeAttrs check [ "formatting" ]);

        treeFmt = {
          projectRootFile = "flake.nix";
          programs = {
            nixfmt.enable = true;
            rustfmt = {
              enable = true;
              edition = "2024";
            };
            taplo.enable = true;
            yamlfmt.enable = true;
          };
          settings = {
            on-unmatched = "warn";
            excludes = [
              "*.lock"
              ".direnv/*"
              ".envrc"
              ".gitignore"
              "result*/*"
              "target/*"
              "LICENSE"
            ];
          };
        };
      in
      {
        packages = {
          server = server.packages.default;
          wasm = wasm.craneLib.mkCargoDerivation (
            wasm.mainArgs
            // {
              inherit (wasm) cargoArtifacts;
              buildPhaseCargoCommand = "wasm-bindgen target/wasm32-unknown-unknown/release/passer.wasm --out-dir pkg";
              installPhaseCommand = "cp -r pkg $out";
            }
          );
        };

        checks = {
          formatting = (treefmt-nix.lib.evalModule pkgs treeFmt).config.build.check self;
        }
        // (prefixCheck "server" server.checks)
        // (prefixCheck "wasm" wasmDev.checks);

        formatter = (treefmt-nix.lib.evalModule pkgs treeFmt).config.build.wrapper;

        devShells = {
          server = server.devShells.default;
          wasm = wasmDev.devShells.default;
        };
      }
    );
}
