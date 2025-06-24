{ pkgs ? import <nixpkgs> {} }:

let
  rustPkgs = pkgs.callPackage ./Cargo.nix { };
in
rustPkgs.rootCrate.build

