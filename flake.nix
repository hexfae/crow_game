{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    {
      nixpkgs,
      fenix,
      ...
    }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      rustToolchain = fenix.packages.${system}.fromToolchainFile {
        file = ./rust-toolchain.toml;
        sha256 = "sha256-SlyeOvqko80434lXjyxxZ7Q7GoA9MUfHQXL0LnHkxks=";
      };
      nativeBuildInputs = with pkgs; [
        rustToolchain
        pkg-config
        wild
        clang
        dioxus-cli
        just
      ];
      buildInputs = with pkgs; [
        alsa-lib
        libudev-zero
        vulkan-loader
        libX11
        libXcursor
        libXi
        libXrandr
        libxkbcommon
        wayland
      ];
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        inherit nativeBuildInputs buildInputs;
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
      };
    };
}
