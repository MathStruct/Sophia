{
  # Sophia is a Rust workspace plus experiments that measure other languages' tools (Julia,
  # Metamath, Lean). This flake builds no package; it sets up the tools, in shells:
  #
  #   nix develop              Rust (cargo, clippy, rustfmt, rust-analyzer), sqlite3 (dbstat),
  #                            zstd, metamath, Julia 1.13 (`julia`, the conformance ground truth)
  #                            and Julia 1.12 (`julia112`, the AD experiment), git, jq, Python
  #   nix develop .#lean       + elan: Lean toolchains are fetched per project by elan
  #
  # Scripts (on the PATH in every shell, also `nix run .#<name>`):
  #
  #   sophia-corpora           clone the real-code corpora (set.mm) into $SOPHIA_CORPORA
  #   sophia-mathlib           clone Mathlib, fetch its build cache, build lean4export
  #                            (several GB; needs the .#lean shell's elan)
  #
  # Large data stays outside the repository, in $SOPHIA_CORPORA (default
  # ~/.cache/sophia-corpora), as before.
  description = "Sophia: a content-addressed store for code in every language";

  # The same revision as LenticulumLM's flake, so its Julia 1.12 is shared, not rebuilt.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/55bfb4d7437bd4e09ab7cba58a43bd68eb310044";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      scripts = pkgs: {
        sophia-corpora = pkgs.writeShellApplication {
          name = "sophia-corpora";
          runtimeInputs = [ pkgs.git ];
          text = ''
            dir="''${SOPHIA_CORPORA:-$HOME/.cache/sophia-corpora}"
            mkdir -p "$dir"
            if [ ! -d "$dir/set.mm/.git" ]; then
              git clone --depth 1 https://github.com/metamath/set.mm "$dir/set.mm"
            else
              echo "set.mm already at $dir/set.mm ($(git -C "$dir/set.mm" rev-parse --short HEAD))"
            fi
          '';
        };

        # Mathlib for B-16 rung 5 (vault/Languages/Lean/Lean Overview.md): the source, its
        # prebuilt .olean cache (`lake exe cache get`), and lean4export at Mathlib's toolchain.
        # Exporting kernel terms is a separate, explicit step (see the message at the end).
        sophia-mathlib = pkgs.writeShellApplication {
          name = "sophia-mathlib";
          runtimeInputs = [ pkgs.git pkgs.curl pkgs.gnutar pkgs.gzip pkgs.zstd ];
          text = ''
            command -v lake >/dev/null || { echo "run this in 'nix develop .#lean' (needs elan)" >&2; exit 1; }
            dir="''${SOPHIA_CORPORA:-$HOME/.cache/sophia-corpora}"
            mkdir -p "$dir"
            if [ ! -d "$dir/mathlib4/.git" ]; then
              git clone --depth 1 https://github.com/leanprover-community/mathlib4 "$dir/mathlib4"
            fi
            (cd "$dir/mathlib4" && lake exe cache get)
            toolchain="$(cat "$dir/mathlib4/lean-toolchain")"
            if [ ! -d "$dir/lean4export/.git" ]; then
              git clone https://github.com/leanprover/lean4export "$dir/lean4export"
            fi
            (cd "$dir/lean4export" && echo "$toolchain" > lean-toolchain && lake build)
            echo "Mathlib at $dir/mathlib4 ($toolchain); lean4export built."
            echo "Export kernel terms with:"
            echo "  cd $dir/mathlib4 && lake env $dir/lean4export/.lake/build/bin/lean4export Mathlib > $dir/mathlib.export"
          '';
        };
      };
    in
    {
      packages = forAllSystems (pkgs: {
        # Julia 1.12, as Lenticulum.jl and the AD experiment use; its install check runs Julia's
        # whole test suite, so it is off (as in LenticulumLM).
        julia112 = pkgs.julia_112-bin.overrideAttrs (_: { doInstallCheck = false; });
      } // scripts pkgs);

      apps = forAllSystems (pkgs: nixpkgs.lib.mapAttrs
        (name: drv: { type = "app"; program = "${drv}/bin/${name}"; })
        (scripts pkgs));

      devShells = forAllSystems (pkgs:
        let
          system = pkgs.stdenv.hostPlatform.system;
          julia112 = pkgs.writeShellScriptBin "julia112" ''
            exec ${self.packages.${system}.julia112}/bin/julia "$@"
          '';
          base = [
            pkgs.rustc
            pkgs.cargo
            pkgs.clippy
            pkgs.rustfmt
            pkgs.rust-analyzer
            pkgs.pkg-config
            pkgs.sqlite # sqlite3 with dbstat, for storage measurements
            pkgs.zstd
            pkgs.metamath # metamath.exe: the oracle for sophia-mm-check
            pkgs.julia-bin # 1.13: methods.jl, the conformance ground truth
            julia112
            pkgs.git
            pkgs.jq
            pkgs.python3
          ] ++ builtins.attrValues (scripts pkgs);

          env = ''
            export SOPHIA_ROOT="$PWD"
            export SOPHIA_CORPORA="''${SOPHIA_CORPORA:-$HOME/.cache/sophia-corpora}"
            export RUST_SRC_PATH="${pkgs.rustPlatform.rustLibSrc}"
          '';

          shell = { name, extra ? [ ], hook ? "" }: pkgs.mkShell {
            inherit name;
            packages = base ++ extra;
            shellHook = env + hook + ''
              echo "Sophia (${name}): $(rustc --version), $(julia --version), corpora in $SOPHIA_CORPORA"
            '';
          };
        in
        {
          default = shell { name = "default"; };
          lean = shell {
            name = "lean";
            extra = [ pkgs.elan ];
            hook = ''
              export ELAN_HOME="''${ELAN_HOME:-$HOME/.elan}"
            '';
          };
        });
    };
}
