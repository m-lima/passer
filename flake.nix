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
            version = "0.2.105";
            hash = "sha256-zLPFFgnqAWq5R2KkaTGAYqVQswfBEYm9x3OPjx8DJRY=";
          };

          cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
            inherit src;
            inherit (src) pname version;
            hash = "sha256-a2X9bzwnMWNt0fTf30qAiJ4noal/ET1jEtf5fBFj5OU=";
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
        wasmBase = helper.lib.rust.helper inputs system ./web/wasm {
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
          web = pkgs.mkYarnPackage {
            nodejs = pkgs.nodejs;

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

            nativeBuildInputs = [ pkgs.writableTmpDirAsHomeHook ];

            doDist = false;

            pkgConfig = {
              node-sass = {
                buildInputs = [
                  (pkgs.python3.withPackages (p: [ p.distutils ]))
                  pkgs.libsass
                  pkgs.pkg-config
                ];

                postInstall = ''
                  LIBSASS_EXT=auto yarn --offline run build
                  rm build/config.gypi
                '';
              };
            };

            yarnPreBuild = ''
              mkdir -p deps/passer/wasm
              cp -r ${wasm}/pkg deps/passer/wasm
              chmod +w deps/passer/wasm

              mkdir -p $HOME/.node-gyp/${pkgs.nodejs.version}
              echo 9 > $HOME/.node-gyp/${pkgs.nodejs.version}/installVersion
              ln -sfv ${pkgs.nodejs}/include $HOME/.node-gyp/${pkgs.nodejs.version}
              export npm_config_nodedir=${pkgs.nodejs}
            '';

            patchPhase = ''
              cp cfg/Config.standalone.ts src/Config.ts
            '';

            configurePhase = ''
              cp -r $node_modules node_modules
              chmod +w node_modules
              rm node_modules/passer_wasm
              mkdir node_modules/passer_wasm
              cp $node_modules/passer_wasm/* node_modules/passer_wasm/.
            '';

            buildPhase = ''
              runHook preBuild
              yarn --offline build
              runHook postBuild
            '';

            installPhase = ''
              runHook preInstall
              mv build $out
              runHook postInstall
            '';
          };
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
