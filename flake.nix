{
  description = "build lavafroth.is-a.dev locally";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in
        {
          devShells.default = pkgs.mkShell rec {
            packages = with pkgs;
            [
              rustc
              cargo
              cmake
              clang
              clang-tools
              onnxruntime
              pkg-config
              alsa-lib
              stdenv.cc.cc.lib
              wget
            ];

            LD_LIBRARY_PATH = "${nixpkgs.lib.makeLibraryPath packages}";
            LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib";
          };
        }
      );
}
