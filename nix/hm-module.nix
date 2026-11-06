# Home Manager module: programs.marky.
self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.marky;
  system = pkgs.stdenv.hostPlatform.system;
  tomlFormat = pkgs.formats.toml { };
  isDarwin = pkgs.stdenv.hostPlatform.isDarwin;

  # Unset typed options are null; leave them out so marky uses its defaults.
  settings = lib.filterAttrs (_: v: v != null) cfg.settings;

  nullOrOption =
    type: description:
    lib.mkOption {
      type = lib.types.nullOr type;
      default = null;
      inherit description;
    };
in
{
  options.programs.marky = {
    enable = lib.mkEnableOption "marky, a Markdown to HTML converter with live preview";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${system}.marky;
      defaultText = lib.literalExpression "marky.packages.\${system}.marky";
      description = "The marky package.";
    };

    settings = lib.mkOption {
      type = lib.types.submodule {
        freeformType = tomlFormat.type;
        options = {
          theme = nullOrOption lib.types.str "Theme to use. See `marky --themes` for the available names.";
          highlight = nullOrOption lib.types.bool "Enable syntax highlighting with highlight.js (`--highlight`).";
          math = nullOrOption lib.types.bool "Enable math rendering with KaTeX (`--math`).";
          diagrams = nullOrOption lib.types.bool "Enable UML diagram rendering with Mermaid (`--diagrams`).";
          include_images = nullOrOption (lib.types.enum [
            "local"
            "remote"
            "all"
          ]) "Include images into the file as base64 (`--include-images`). null leaves images as links.";
          optimize_images = nullOrOption lib.types.bool "Optimize included images to make them smaller (`--optimize-images`).";
          port = nullOrOption lib.types.port "Port of the live server (`--port`). marky's default is 8080.";
          open = nullOrOption lib.types.bool "Open the output file in the default app (`--open`).";
        };
      };
      default = { };
      example = lib.literalExpression ''
        {
          theme = "air";
          highlight = true;
          include_images = "local";
          port = 3000;
        }
      '';
      description = ''
        Settings for marky's `config.toml`: `$XDG_CONFIG_HOME/marky/config.toml`
        on Linux, `~/Library/Application Support/marky/config.toml` on macOS.
        The file is written only when a setting is set. Flags on the command
        line take precedence. marky fails on unknown keys, so only add keys
        your marky version knows.
      '';
    };
  };

  config = lib.mkIf cfg.enable (
    let
      configFile = lib.mkIf (settings != { }) {
        source = tomlFormat.generate "marky-config.toml" settings;
      };
    in
    {
      home.packages = [ cfg.package ];

      # marky reads dirs::config_dir()/marky, which is not XDG on macOS.
      xdg.configFile."marky/config.toml" = lib.mkIf (!isDarwin) configFile;
      home.file."Library/Application Support/marky/config.toml" = lib.mkIf isDarwin configFile;
    }
  );
}
