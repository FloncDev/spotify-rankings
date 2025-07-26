{ pkgs, ... }:
{
  languages.rust = {
    enable = true;
    # channel = "nightly";
  };

  languages.typescript.enable = true;

  languages.javascript = {
    enable = true;
    bun = {
      enable = true;
      install.enable = true;
    };
    npm.enable = true;
  };

  # services.postgres = {
  #   enable = true;
  #   listen_addresses = "*";
  #   initialDatabases = [
  #     {
  #       name = "postgres";
  #       user = "postgres";
  #       pass = "postgres";
  #     }
  #   ];
  # };

  packages = with pkgs; [
    lld
    # tinymist
    # typstyle
    # typst
    sqlx-cli
    cargo-watch
    pkg-config
    openssl
  ];
}
