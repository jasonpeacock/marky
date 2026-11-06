{
  lib,
  stdenv,
  rustPlatform,
  installShellFiles,
}:
let
  cargo = lib.importTOML ../Cargo.toml;
in
rustPlatform.buildRustPackage {
  pname = "marky";
  inherit (cargo.package) version;

  # templates/ and vendor/ are embedded into the binary with include_dir!.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../templates
      ../vendor
    ];
  };
  cargoLock.lockFile = ../Cargo.lock;

  doCheck = true;

  nativeBuildInputs = [ installShellFiles ];

  # Completions come from running marky itself, so skip them when
  # cross-compiling to a platform the build machine cannot run.
  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    installShellCompletion --cmd marky \
      --bash <($out/bin/marky --completion bash) \
      --fish <($out/bin/marky --completion fish) \
      --zsh <($out/bin/marky --completion zsh)
  '';

  meta = {
    description = cargo.package.description;
    # The fork, which ships this flake; Cargo.toml still names upstream.
    homepage = "https://github.com/jasonpeacock/marky";
    license = lib.licenses.mit;
    mainProgram = "marky";
    platforms = lib.platforms.unix;
  };
}
