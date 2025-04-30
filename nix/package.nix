{
  rustPlatform,
  installShellFiles,
  pkg-config,
  openssl,
}:

rustPlatform.buildRustPackage {
  pname = "arc";
  version = "0.2.0";

  nativeBuildInputs = [
    installShellFiles
    pkg-config
  ];

  buildInputs = [
    openssl
  ];

  src = ./..;
  cargoHash = "sha256-TEnHrmNw0AuXNEP1W0vlVqtAbQ43fiZP9NblL9xPsNk=";

  postInstall = ''
    installShellCompletion --cmd arc \
      --bash <($out/bin/arc completion bash) \
      --fish <($out/bin/arc completion fish) \
      --zsh <($out/bin/arc completion zsh)
  '';

  meta = {
    mainProgram = "arc";
    description = "Cli for configuring arub switches";
  };
}
