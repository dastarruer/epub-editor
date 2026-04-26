{
  description = "A Nix-flake-based Rust + SvelteKit (Tauri) development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    fenix,
    ...
  }: let
    supportedSystems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin"];
    forEachSupportedSystem = f:
      nixpkgs.lib.genAttrs supportedSystems (system:
        f rec {
          inherit system;
          pkgs = import nixpkgs {
            inherit system;
            overlays = [self.overlays.default];
          };
        });
  in {
    overlays.default = final: prev: {
      rustToolchain = fenix.packages.${prev.system}.stable.withComponents [
        "cargo"
        "clippy"
        "rust-src"
        "rustc"
        "rustfmt"
      ];
    };

    devShells = forEachSupportedSystem ({
      pkgs,
      system,
    }: let
      # Libraries required for both compilation and runtime
      runtimeLibs = with pkgs; [
        webkitgtk_4_1
        gtk3
        cairo
        gdk-pixbuf
        glib
        dbus
        openssl
        librsvg
        libsoup_3
      ];
    in {
      default = pkgs.mkShell {
        # nativeBuildInputs: Tools that run on the host during build
        nativeBuildInputs = with pkgs; [
          pkg-config
          rustToolchain
          cargo-tauri
          cargo-deny
          cargo-edit
          cargo-watch
          nixd
          alejandra
        ];

        # buildInputs: Libraries the app links against
        buildInputs = with pkgs;
          [
            nodejs_22 # Stick to 22 for stability with current OpenSSL
            pnpm
            svelte-language-server
            eslint
          ]
          ++ runtimeLibs;

        env = {
          RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";

          # Automatically map the runtime libs for the linker
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;

          # Ensures GTK settings, themes, and icons are found
          XDG_DATA_DIRS = with pkgs; "${gsettings-desktop-schemas}/share/gsettings-schemas/${gsettings-desktop-schemas.name}:${gtk3}/share/gsettings-schemas/${gtk3.name}:$XDG_DATA_DIRS";
        };
      };
    });
  };
}
