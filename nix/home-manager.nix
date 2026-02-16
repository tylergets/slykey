{self}: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.slykey;
  yaml = pkgs.formats.yaml {};
  generatedConfig =
    yaml.generate "slykey-config.yaml" ({
        expansions = cfg.expansions;
      }
      // lib.optionalAttrs (cfg.matchBehavior != null) {
        match_behavior = cfg.matchBehavior;
      }
      // lib.optionalAttrs (cfg.boundaryChars != null) {
        boundary_chars = cfg.boundaryChars;
      });
in {
  options.programs.slykey = {
    enable = lib.mkEnableOption "slykey text expansion service";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}.default;
      defaultText = lib.literalExpression "self.packages.\${pkgs.system}.default";
      description = "The slykey package to run.";
    };

    expansions = lib.mkOption {
      type = lib.types.listOf (lib.types.submodule {
        options = {
          trigger = lib.mkOption {
            type = lib.types.str;
            description = "Typed trigger text.";
            example = "sig;";
          };
          expansion = lib.mkOption {
            type = lib.types.str;
            description = "Text or macro sequence to emit.";
            example = "Thanks, Tyler{{KEY:ENTER}}";
          };
        };
      });
      default = [];
      description = "Expansion rules written into the generated slykey YAML config.";
    };

    matchBehavior = lib.mkOption {
      type = lib.types.nullOr (lib.types.enum [
        "immediate"
        "boundary"
      ]);
      default = "immediate";
      description = "slykey match behavior for trigger activation.";
    };

    boundaryChars = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Optional boundary character set used when matchBehavior is boundary.";
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.expansions != [];
        message = "programs.slykey.expansions must include at least one rule when enabled.";
      }
    ];

    home.packages = [cfg.package];

    systemd.user.services.slykey = {
      Unit = {
        Description = "slykey text expansion daemon";
        After = ["graphical-session.target"];
        PartOf = ["graphical-session.target"];
      };

      Service = {
        ExecStart = "${lib.getExe cfg.package} --config ${generatedConfig} run";
        Restart = "on-failure";
        RestartSec = 2;
      };

      Install = {
        WantedBy = ["graphical-session.target"];
      };
    };
  };
}
