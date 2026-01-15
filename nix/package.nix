{ inputs, ... }: {
  perSystem = { pkgs, self', ... }: {
    packages.default = self'.packages.attic-trusted-publisher;
    packages.attic-trusted-publisher =
      let craneLib = inputs.crane.mkLib pkgs;
      in craneLib.buildPackage {
        name = "attic-trusted-publisher";
        src = craneLib.cleanCargoSource ./..;
        meta.mainProgram = "attic-trusted-publisher";
      };

    devShells.attic-trusted-publisher = pkgs.mkShell {
      inputsFrom = [ self'.packages.attic-trusted-publisher ];
      nativeBuildInputs = [ pkgs.rustfmt ];
    };
  };
}
