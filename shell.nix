{ pkgs ? import <nixpkgs> { }
, unstable ? import <unstable> { }
}:

pkgs.mkShell {
  nativeBuildInputs = [
    unstable.duckdb
    pkgs.mold
    pkgs.rust-analyzer
    pkgs.rustup
  ];
}
