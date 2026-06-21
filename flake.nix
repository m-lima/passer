{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
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
            version = "0.2.108";
            hash = "sha256-UsuxILm1G6PkmVw0I/JF12CRltAfCJQFOaT4hFwvR8E=";
          };

          cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
            inherit src;
            inherit (src) pname version;
            hash = "sha256-iqQiWbsKlLBiJFeqIYiXo3cqxGLSjNM8SOWXGM9u43E=";
          };
        };
        server =
          (helper.lib.rust.helper inputs system ./server {
            allowFilesets = [ ./server/res ];
          }).outputs;
        wasmDev =
          (helper.lib.rust.helper inputs system ./wasm {
            binary = false;
            mega = false;
            extraToolchains = fenixPkgs: [
              fenixPkgs.targets.wasm32-unknown-unknown.stable.rust-std
            ];
            nativeBuildInputs = pkgs: [ bindgen ];
          }).outputs;
        wasmBase = helper.lib.rust.helper inputs system ./wasm {
          binary = false;
          mega = false;
          extraToolchains = fenixPkgs: [
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
        wasm =
          let
            name = "${wasmBase.mainArtifact.pname}";
            version = "${wasmBase.mainArtifact.version}";
          in
          wasmBase.craneLib.mkCargoDerivation (
            wasmBase.mainArgs
            // {
              cargoArtifacts = wasmBase.mainArtifact;
              buildPhaseCargoCommand = "wasm-bindgen target/lib/${name}.wasm --out-dir pkg --typescript --target bundler";
              installPhaseCommand = ''
                mkdir -p $out
                cp -r pkg $out/pkg
                cat > $out/pkg/package.json <<EOF
                {
                  "name": "${name}",
                  "type": "module",
                  "version": "${version}",
                  "files": [
                    "${name}_bg.wasm",
                    "${name}.js",
                    "${name}_bg.js",
                    "${name}.d.ts"
                  ],
                  "main": "${name}.js",
                  "types": "${name}.d.ts",
                  "sideEffects": [
                    "./${name}.js",
                    "./snippets/*"
                  ]
                }
                EOF
              '';
            }
          );

        commonWeb =
          let
            package = builtins.fromJSON (builtins.readFile ./web/package.json);
          in
          {
            pname = package.name;
            version = package.version;

            nativeBuildInputs = [
              pkgs.nodejs
              pkgs.yarnConfigHook
              pkgs.yarnBuildHook
            ];

            src = pkgs.lib.fileset.toSource {
              root = ./web;
              fileset = pkgs.lib.fileset.unions [
                ./web/package.json
                ./web/yarn.lock
                ./web/tsconfig.json
                ./web/config-overrides.js
                ./web/src
                ./web/public
                ./web/cfg
              ];
            };

            offlineCache = pkgs.fetchYarnDeps {
              yarnLock = ./web/yarn.lock;
              hash = "sha256-BBLxRvLTCelHnbWPLe0pFA4B1mprgVsP3ebVuRsVh4c=";
            };

            doCheck = false;

            patchPhase = ''
              cp cfg/Config.bundle.ts src/Config.ts
              cp -a ${wasm}/pkg ./wasm
            '';

          };
        webChecks = {
          lint = pkgs.stdenvNoCC.mkDerivation (
            commonWeb
            // {
              doCheck = true;
              dontBuild = true;

              installPhase = "mkdir -p $out";
            }
          );
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
            beautysh.enable = true;
            rustfmt = {
              enable = true;
              edition = "2024";
            };
            taplo.enable = true;
            xmllint.enable = true;
            prettier.enable = true;
          };
          settings = {
            prettier = builtins.fromJSON (builtins.readFile ./web/.prettierrc.json);
            on-unmatched = "warn";
            excludes = [
              "**/.direnv/*"
              "**/.envrc"
              "**/.gitignore"
              "*.ico"
              "*.lock"
              "*.png"
              ".direnv/*"
              ".dockerignore"
              ".envrc"
              ".gitignore"
              "Dockerfile*"
              "LICENSE"
              "result*/*"
              "server/res/*"
              "target/*"
              "web/.direnv/*"
              "web/.envrc"
              "web/public/robots.txt"
            ];
          };
        };
      in
      {
        packages = {
          server = server.packages.default;
          wasm = wasm;
          web = pkgs.stdenvNoCC.mkDerivation (
            commonWeb
            // {
              installPhase = ''
                runHook preInstall
                mv build $out
                runHook postInstall
              '';
            }
          );
        };

        checks = {
          formatting = (treefmt-nix.lib.evalModule pkgs treeFmt).config.build.check self;
        }
        // (prefixCheck "server" server.checks)
        // (prefixCheck "wasm" wasmDev.checks)
        // (prefixCheck "web" webChecks);

        formatter = (treefmt-nix.lib.evalModule pkgs treeFmt).config.build.wrapper;

        devShells = {
          server = server.devShells.default;
          wasm = wasmDev.devShells.default;
          web = pkgs.mkShell {
            buildInputs = [
              pkgs.yarn
              (pkgs.python3.withPackages (p: [ p.distutils ]))
            ];
          };
        };
      }
    );
}
