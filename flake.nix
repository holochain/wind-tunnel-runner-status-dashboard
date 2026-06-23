{
  description = "Flake for Holochain testing";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";

    flake-parts.url = "github:hercules-ci/flake-parts";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ flake-parts, rust-overlay, nixpkgs, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = ["x86_64-linux" "aarch64-darwin"];

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
              pkgs.go
              pkgs.pulumi
              pkgs.pulumiPackages.pulumi-go
            ];
          };
        };
    };
  };
}
