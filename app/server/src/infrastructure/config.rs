use std::{
    env, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct ServerSettings {
    pub app: AppSettings,
    pub server: HttpServerSettings,
    pub log: LogSettings,
    pub database: DatabaseSettings,
    pub jwt_secret: String,
    pub redis_url: Option<String>,
}

impl ServerSettings {
    pub fn load() -> anyhow::Result<Self> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dotenv_path = env::var("APP_DOTENV_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| manifest_dir.join(".env"));

        let _ = dotenvy::from_path(&dotenv_path);

        let environment = env::var("APP_ENV")
            .or_else(|_| env::var("APP__ENVIRONMENT"))
            .unwrap_or_else(|_| "dev".to_owned());
        let config_dir = env::var("APP_CONFIG_DIR")
            .map(|value| resolve_path_from_base(value, &manifest_dir))
            .unwrap_or_else(|_| manifest_dir.join("config"));

        let mut settings = Self::default_for(&environment);
        settings.apply(load_config_file(config_dir.join("default.toml"))?);
        settings.apply(load_config_file(
            config_dir.join(format!("{environment}.toml")),
        )?);
        settings.apply_env_overrides();
        settings.database.url = env::var("DATABASE_URL").ok();
        settings.jwt_secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| "change-me-in-production".to_owned());
        settings.redis_url = env::var("REDIS_URL").ok();

        Ok(settings)
    }

    fn default_for(environment: &str) -> Self {
        Self {
            app: AppSettings {
                name: "voice-room-server".to_owned(),
                environment: environment.to_owned(),
            },
            server: HttpServerSettings {
                host: "0.0.0.0".to_owned(),
                port: 3000,
            },
            log: LogSettings {
                level: "info".to_owned(),
                format: "json".to_owned(),
                service_name: "voice-room-server".to_owned(),
            },
            database: DatabaseSettings {
                url: None,
                max_connections: 10,
                connect_timeout_secs: 5,
            },
            jwt_secret: "change-me-in-production".to_owned(),
            redis_url: None,
        }
    }

    fn apply(&mut self, file: ConfigFile) {
        if let Some(app) = file.app {
            if let Some(name) = app.name {
                self.app.name = name;
            }
            if let Some(environment) = app.environment {
                self.app.environment = environment;
            }
        }

        if let Some(server) = file.server {
            if let Some(host) = server.host {
                self.server.host = host;
            }
            if let Some(port) = server.port {
                self.server.port = port;
            }
        }

        if let Some(log) = file.log {
            if let Some(level) = log.level {
                self.log.level = level;
            }
            if let Some(format) = log.format {
                self.log.format = format;
            }
            if let Some(service_name) = log.service_name {
                self.log.service_name = service_name;
            }
        }

        if let Some(database) = file.database {
            if let Some(max_connections) = database.max_connections {
                self.database.max_connections = max_connections;
            }
            if let Some(connect_timeout_secs) = database.connect_timeout_secs {
                self.database.connect_timeout_secs = connect_timeout_secs;
            }
        }
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(host) = env::var("APP__SERVER__HOST") {
            self.server.host = host;
        }

        if let Ok(port) = env::var("APP__SERVER__PORT") {
            if let Ok(port) = port.parse() {
                self.server.port = port;
            }
        }

        if let Ok(level) = env::var("APP__LOG__LEVEL") {
            self.log.level = level;
        }

        if let Ok(format) = env::var("APP__LOG__FORMAT") {
            self.log.format = format;
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct AppSettings {
    pub name: String,
    pub environment: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HttpServerSettings {
    pub host: String,
    pub port: u16,
}

impl HttpServerSettings {
    pub fn bind_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(format!("{}:{}", self.host, self.port).parse()?)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct LogSettings {
    pub level: String,
    pub format: String,
    pub service_name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseSettings {
    #[serde(default)]
    pub url: Option<String>,
    pub max_connections: u32,
    pub connect_timeout_secs: u64,
}

#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    app: Option<AppSettingsFile>,
    server: Option<HttpServerSettingsFile>,
    log: Option<LogSettingsFile>,
    database: Option<DatabaseSettingsFile>,
}

#[derive(Debug, Deserialize)]
struct AppSettingsFile {
    name: Option<String>,
    environment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HttpServerSettingsFile {
    host: Option<String>,
    port: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct LogSettingsFile {
    level: Option<String>,
    format: Option<String>,
    service_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DatabaseSettingsFile {
    max_connections: Option<u32>,
    connect_timeout_secs: Option<u64>,
}

fn load_config_file<P>(path: P) -> anyhow::Result<ConfigFile>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    if !path.exists() {
        return Ok(ConfigFile::default());
    }

    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

fn resolve_path_from_base(path: String, base: &Path) -> PathBuf {
    let path = PathBuf::from(path);

    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}
