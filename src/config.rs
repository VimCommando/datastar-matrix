use clap::Parser;

pub const DEFAULT_FPS: f32 = 60.0;

#[derive(Debug, Clone, Parser)]
#[command(name = "datastar-matrix")]
#[command(about = "Matrix rain in terminal and browser", long_about = None)]
pub struct Config {
    #[arg(long = "fps", default_value_t = DEFAULT_FPS)]
    pub target_fps: f32,
    #[arg(long = "port")]
    pub port: Option<u16>,
}

impl Config {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }

    #[cfg(feature = "web")]
    pub fn web_enabled(&self) -> bool {
        true
    }

    #[cfg(not(feature = "web"))]
    pub fn web_enabled(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn defaults_to_sixty_fps() {
        let cfg = Config::try_parse_from(["datastar-matrix"]).expect("default parse should work");
        assert_eq!(cfg.target_fps, 60.0);
        assert_eq!(cfg.port, None);
    }

    #[test]
    fn parses_fps_and_port() {
        let cfg = Config::try_parse_from(["datastar-matrix", "--fps", "30", "--port", "8123"])
            .expect("parse should work");
        assert_eq!(cfg.target_fps, 30.0);
        assert_eq!(cfg.port, Some(8123));
    }
}
