pub struct Env {
    pub jwt_secret: String,
    pub access_token_expiration: u64,
    pub refresh_token_expiration: u64,
    pub database_url: String,
    pub redis_url: String,
    pub frontend_url: String,
    pub ip: String,
    pub port: u16,
}

impl Env {
    fn new() -> Self {
        let jwt_secret = std::env::var("SECRET_KEY")
            .expect("SECRET_KEY must be set in .env file or environment variable");

        let access_token_expiration = std::env::var("ACCESS_TOKEN_EXPIRATION")
            .unwrap_or_else(|_| "900".to_string())
            .parse::<u64>()
            .expect("ACCESS_TOKEN_EXPIRATION must be a valid u64 integer");
        let refresh_token_expiration = std::env::var("REFRESH_TOKEN_EXPIRATION")
            .unwrap_or_else(|_| "604800".to_string())
            .parse::<u64>()
            .expect("REFRESH_TOKEN_EXPIRATION must be a valid u64 integer");

        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set in .env file or environment variable");
        let redis_url = std::env::var("REDIS_URL")
            .expect("REDIS_URL must be set in .env file or environment variable");

        let frontend_url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
        let ip = std::env::var("IP").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .expect("PORT must be a valid u16 integer");
        Env {
            jwt_secret,
            access_token_expiration,
            refresh_token_expiration,
            database_url,
            redis_url,
            frontend_url,
            ip,
            port,
        }
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}
