{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    qahq.url = "github:mlavrinenko/qahq";
  };

  outputs =
    {
      flake-utils,
      naersk,
      nixpkgs,
      qahq,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        naersk' = pkgs.callPackage naersk { };

      in
      {
        # For `nix build` & `nix run`:
        packages.default = naersk'.buildPackage {
          src = ./.;
        };

        # For `nix develop`:
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            qahq.packages.${system}.linecop
          ] ++ (with pkgs; [
            rustc
            cargo
            cargo-machete
            cargo-tarpaulin
            clippy
            rustfmt
            just
            parallel
            nixd
            rust-analyzer
          ]);
        };
      }
    );
}
