{ pkgs ? import <nixpkgs> { }
, unstable ? import <unstable> { }
}:

pkgs.mkShell {
  nativeBuildInputs = [
    pkgs.duckdb
    pkgs.rust-analyzer
    pkgs.rustup
  ];
}
