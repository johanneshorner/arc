{
  rustPlatform,
  installShellFiles,
  pkg-config,
  openssl,
}:

rustPlatform.buildRustPackage {
  pname = "arc";
  version = "0.1.0";

  nativeBuildInputs = [
    installShellFiles
    pkg-config
  ];

  buildInputs = [
    openssl
  ];

  src = ./..;
  cargoHash = "sha256-9Ih4M/sxBJ92ZOl2n9QzQ2S0UiuCQwW5EEqCw2yQDl4=";

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
