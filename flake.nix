{
  description = "Rhythm Rail — path-based rhythm game dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };

        bevyDeps = with pkgs; [
          vulkan-loader
          vulkan-headers
          vulkan-tools
          vulkan-validation-layers

          libx11
          libxcursor
          libxi
          libxrandr

          wayland
          libxkbcommon

          alsa-lib

          udev
          libGL
        ];

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          mold
          clang
        ];

        libraryPath = pkgs.lib.makeLibraryPath bevyDeps;
      in
      {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs;
          buildInputs = bevyDeps;

          shellHook = ''
            export LD_LIBRARY_PATH="${libraryPath}:$LD_LIBRARY_PATH"
            export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
            echo "Rhythm Rail dev environment loaded"
            echo "Rust: $(rustc --version)"
          '';

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };
      }
    );
}
