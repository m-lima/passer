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
      flake-utils,
      helper,
      ...
    }@inputs:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        server =
          (helper.lib.rust.helper inputs system ./server {
            allowFilesets = [ ./server/res ];
          }).outputs;
        wasm = (helper.lib.rust.helper inputs system ./web/wasm { }).outputs;
      in
      {
        packages = {
          server = server.packages.default;
          wasm = wasm.packages.default;
        };

        devShells = {
          server = server.devShells.default;
          wasm = wasm.devShells.default;
        };
      }
    );
}
