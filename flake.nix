{
  description = "Rust Development Shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        libraries = with pkgs; [
          #Add Liberaries here
        ];

        packages = with pkgs; [
          lldb_19
          pkg-config
          (rust-bin.selectLatestNightlyWith (
            toolchain:
            toolchain.default.override {
              extensions = [
                "rust-src"
                "rust-analyzer"
              ];
            }
          ))
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = packages;

          shellHook = ''
            export HELLO_RATATUI_CONFIG=`pwd`/.config
            export HELLO_RATATUI_DATA=`pwd`/.data
            export HELLO_RATATUI_LOG_LEVEL=debug
          '';
        };
      }
    );
}
