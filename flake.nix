{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/25.11";
    crane.url = "github:ipetkov/crane";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs: inputs.flake-parts.lib.mkFlake { inherit inputs; } ({ self, moduleWithSystem, ... }: {
    systems = [ "x86_64-linux" ];
    imports = [
      ./nix/package.nix
      ./nix/nixos.nix
      ./nix/checks
    ];
  });
}
