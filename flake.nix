{
  description = "Flake for Holochain testing";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";

    holonix.url = "github:holochain/holonix?ref=main-0.6";

    flake-parts.url = "github:hercules-ci/flake-parts";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ flake-parts, rust-overlay, nixpkgs, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = builtins.attrNames inputs.holonix.devShells;

    perSystem = { inputs', pkgs, system, config, ... }: {
      _module.args.pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };

      formatter = pkgs.nixpkgs-fmt;

      devShells =
        let
          rustFromToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        {
          default = pkgs.mkShell {
            packages = [
              rustFromToolchain
            ];
          };
        };
    };
  };
}
