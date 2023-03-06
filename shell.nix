{ pkgs ? import <nixpkgs> { }
, unstable ? import <unstable> { }
}:

pkgs.mkShell {
  nativeBuildInputs = [
    unstable.duckdb
    pkgs.rust-analyzer
    pkgs.rustup
  ];
}
