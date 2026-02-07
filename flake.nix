{
  description = "Open Speech-to-Text recording tool with real-time volume metering and transcription";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        packages = {
          ostt = pkgs.rustPlatform.buildRustPackage {
            pname = "ostt";
            version = "0.0.5";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = with pkgs; [
              pkg-config
              makeWrapper
            ];

            buildInputs = with pkgs;
              [
                openssl
                alsa-lib
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
                darwin.apple_sdk.frameworks.AudioUnit
                darwin.apple_sdk.frameworks.CoreAudio
                darwin.apple_sdk.frameworks.CoreFoundation
              ];

            postInstall = ''
              wrapProgram $out/bin/ostt \
                --prefix PATH : ${pkgs.lib.makeBinPath [
                pkgs.ffmpeg
                pkgs.wl-clipboard
                pkgs.xclip
              ]}
            '';

            meta = with pkgs.lib; {
              description = "Open Speech-to-Text recording tool with real-time volume metering and transcription";
              homepage = "https://github.com/kristoferlund/ostt";
              license = licenses.mit;
              maintainers = [];
              mainProgram = "ostt";
            };
          };

          default = self.packages.${system}.ostt;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rust-analyzer
            pkg-config
            openssl
            alsa-lib
            ffmpeg
            wl-clipboard
            xclip
          ];
        };
      }
    );
}
