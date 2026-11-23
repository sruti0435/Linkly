use std::{env, io};

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: Option<String>,
    pub jwt_secret: String,
    pub base_url: String,
    pub api_host: String,
    pub api_port: u16,
    pub frontend_origin: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "JWT_SECRET must be at least 32 bytes",
            )
            .into());
        }

        Ok(Self {
            database_url: required("DATABASE_URL")?,
            redis_url: env::var("REDIS_URL").ok().filter(|url| !url.is_empty()),
            jwt_secret,
            base_url: required("BASE_URL")?.trim_end_matches('/').to_owned(),
            api_host: env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_owned()),
            api_port: env::var("API_PORT")
                .or_else(|_| env::var("PORT"))
                .unwrap_or_else(|_| "8080".to_owned())
                .parse()?,
            frontend_origin: env::var("FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".to_owned()),
        })
    }
}

fn required(name: &str) -> Result<String, io::Error> {
    env::var(name).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("required environment variable {name} is missing"),
        )
    })
}
