{
  description = "devshell for github:lavafroth/tempest";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let pkgs = nixpkgs.legacyPackages.${system};
        architecture = builtins.elemAt (pkgs.lib.splitString "-" system) 0;
         libvosk = pkgs.stdenv.mkDerivation {
          name = "libvosk";
          pname = "libvosk";

          src = pkgs.fetchurl {
            url = "https://github.com/alphacep/vosk-api/releases/download/v0.3.45/vosk-linux-${architecture}-0.3.45.zip";
            sha256 = "sha256-u9yO2FxDl59kQxQoiXcOqVy/vFbP+1xdzXOvqHXF+7I=";
          };

          nativeBuildInputs = with pkgs; [ unzip ];
          unpackPhase = "unzip $src";

          installPhase = ''
            mkdir -p $out/lib
            mv vosk-linux-x86_64-0.3.45/* $out/lib
          '';
        };
         in
        {
          devShells.default = pkgs.mkShell rec {

            packages = with pkgs;
            [
              libvosk
              rustc
              cargo
              cmake
              clang
              pkg-config
              alsa-lib
              stdenv.cc.cc.lib
              openssl.dev
            ];

            LD_LIBRARY_PATH = "${nixpkgs.lib.makeLibraryPath packages}";
            LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib";
          };
        }
      );
}
