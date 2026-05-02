{
  description = "Devshell for this project";

  nixConfig = {
    extra-substituters = [
      "https://fenix.cachix.org"
    ];
    extra-trusted-public-keys = [
      "fenix.cachix.org-1:ecJhr+RdYEdcVgUkjruiYhjbBloIEGov7bos90cZi0Q="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    fenix.url = "github:nix-community/fenix";

    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    ...
  } @ inputs: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [inputs.fenix.overlays.default];
    };

    lib = pkgs.lib;

    rust-toolchain = pkgs.fenix.fromToolchainFile {
      file = ./src-tauri/rust-toolchain.toml;
      sha256 = "sha256-Qxt8XAuaUR2OMdKbN4u8dBJOhSHxS+uS06Wl9+flVEk=";
    };

    pre-commit-check = inputs.git-hooks.lib.${system}.run {
      src = ./.;

      # GIT HOOKS GO HERE
      # See https://devenv.sh/git-hooks/ for how to configure hooks
      # To get the root of the project, use the following command as a workaround: $(git rev-parse --show-toplevel)
      # See https://github.com/NixOS/nix/issues/8034#issuecomment-3366842508 for more info
      hooks = let
        pnpm-lint-wrapper = pkgs.writeShellApplication {
          name = "prettier";
          runtimeInputs = [pkgs.pnpm];

          text = ''
            cd "$(git rev-parse --show-toplevel)"/frontend
            pnpm run lint
          '';
        };

        pnpm-check-wrapper = pkgs.writeShellApplication {
          name = "pnpm-check";
          runtimeInputs = [pkgs.pnpm];

          text = ''
            cd "$(git rev-parse --show-toplevel)"/frontend
            pnpm run check
          '';
        };
      in {
        alejandra.enable = true;

        pnpm-lint = {
          enable = false;
          name = "pnpm-lint";
          entry = "${lib.getExe pnpm-lint-wrapper}";

          files = "^frontend/.*\\.(${
            builtins.concatStringsSep "|" [
              "js"
              "ts"
              "json"
              "yaml"
              "svelte"
            ]
          })$";
        };

        pnpm-check = {
          enable = false;
          name = "svelte-check";
          entry = "${lib.getExe pnpm-check-wrapper}";
          pass_filenames = false;

          files = "^frontend/.*\\.(${
            builtins.concatStringsSep "|" [
              "js"
              "ts"
              "svelte"
            ]
          })$";
        };

        clippy = {
          enable = false;

          packageOverrides = {
            cargo = rust-toolchain;
            clippy = rust-toolchain;
          };

          settings = {
            allFeatures = true;
            denyWarnings = true;
            extraArgs = "--manifest-path src-tauri/Cargo.toml";
          };
        };

        rustfmt = {
          enable = true;

          packageOverrides = {
            cargo = rust-toolchain;
            rustfmt = rust-toolchain;
          };

          settings = {
            check = true;
            manifest-path = "src-tauri/Cargo.toml";
          };
        };

        check-toml.enable = true;
        taplo.enable = true;

        prettier = {
          enable = true;

          # pnpm-lint will handle files in frontend/
          excludes = ["^frontend/.*"];
        };

        markdownlint.enable = true;
      };
    };
  in {
    devShells.${system}.default = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [
        # MARKDOWN
        markdownlint-cli # Formatter

        # NIX
        nixd # LSP
        alejandra # Formatter

        # NODE
        nodejs_25
        pnpm

        # RUST
        rust-toolchain
        rust-analyzer-nightly
        cargo-tauri
        pkg-config
        wrapGAppsHook4
      ];

      buildInputs = with pkgs; [
        glib
        gtk3
        libsoup_3
        librsvg
        webkitgtk_4_1
        cairo
        pango
        atk
        gdk-pixbuf
        harfbuzz
        dbus
        openssl
      ];

      shellHook = ''
        # Install pre-commit hooks in the shell hook
        ${pre-commit-check}

        # TODO: Move this list of packages to a variable
        export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath (with pkgs; [
          glib
          gtk3
          libsoup_3
          librsvg
          webkitgtk_4_1
          cairo
          pango
          atk
          gdk-pixbuf
          harfbuzz
          dbus
          openssl
        ])}:$LD_LIBRARY_PATH

        export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH" # Needed on Wayland to report the correct display scale
      '';
    };
  };
}
