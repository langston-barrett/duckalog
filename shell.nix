{ pkgs ? import <nixpkgs> { }
, unstable ? import <unstable> { }
}:

pkgs.mkShell {
  nativeBuildInputs = [
    pkgs.duckdb
    pkgs.mold
    pkgs.pkg-config
    pkgs.rust-analyzer
    pkgs.rustup
  ];
  # Wasn't able to make any of this work...
  # PKG_CONFIG_PATH = "${pkgs.sqlite.dev}/lib/pkgconfig";
  # pkgs.sqlite
  # pkgs.sqlite.dev
}
