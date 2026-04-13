{
  description = "DAW - Digital Audio Workstation with Dioxus UI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, crane, fenix, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        # Fenix toolchain
        toolchain = fenix.packages.${system}.complete.withComponents [
          "cargo"
          "clippy"
          "rust-src"
          "rustfmt"
          "rustc"
        ];
        
        # Crane library
        craneLib = crane.mkLib pkgs;
        
        # Common arguments for crane
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          
          nativeBuildInputs = with pkgs; [
            toolchain
            pkg-config
            alsa-lib
            jack2
            wrapGAppsHook4
            glib.dev
            gtk3.dev
            webkitgtk_4_1.dev
            libsoup_3.dev
            cairo.dev
            pango.dev
            atk.dev
            gdk-pixbuf.dev
            harfbuzz.dev
            xdotool
          ];
          
          buildInputs = with pkgs; [
            # GUI libraries for Dioxus desktop
            libxkbcommon
            libGL
            libX11
            libXcursor
            libXrandr
            libXi
            wayland
            wayland-protocols
            
            # GTK/WebKit for Dioxus desktop (WebView)
            glib
            gtk3
            webkitgtk_4_1
            libsoup_3
            cairo
            pango
            atk
            gdk-pixbuf
            harfbuzz
            xdotool
          ];
        };
        
        # Build dependencies only
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        
        # Build the actual crate
        dawPackage = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "-p daw_dioxus";
        });
        
      in {
        packages = {
          default = dawPackage;
          daw = dawPackage;
        };
        
        checks = {
          # Run clippy
          daw-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "-p daw_dioxus --all-targets -- --deny warnings";
          });
          
          # Run fmt check
          daw-fmt = craneLib.cargoFmt {
            src = craneLib.cleanCargoSource ./.;
          };
          
          # Run tests
          daw-tests = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            cargoNextestExtraArgs = "-p daw_dioxus";
            partitions = 1;
            partitionType = "count";
          });
        };
        
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = with pkgs; [
            # Audio/audio hardware tools
            usbutils
            alsa-utils
            
            # GUI libraries for Dioxus
            libxkbcommon
            libGL
            libX11
            libXcursor
            libXrandr
            libXi
            wayland
            wayland-protocols
            
            # GTK/WebKit for Dioxus desktop
            glib
            gtk3
            webkitgtk_4_1
            libsoup_3
            cairo
            pango
            atk
            gdk-pixbuf
            harfbuzz
            xdotool
            
            # Additional tools
            rust-analyzer
            cargo-watch
            cargo-edit
            cargo-deny
            cargo-audit
            cargo-expand  # Useful for macro debugging
          ];
          
          # Set environment variables for GUI
          LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [
            libxkbcommon
            libGL
            libX11
            libXcursor
            libXrandr
            libXi
            wayland
            glib
            gtk3
            webkitgtk_4_1
            libsoup_3
            cairo
            pango
            atk
            gdk-pixbuf
            xdotool
          ];
        };
      });
}
