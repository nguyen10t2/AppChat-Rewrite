pub mod user {
    pub mod schema;
    pub mod model;
    pub mod repository;
    pub mod repository_pg;
    pub mod handle;
    pub mod service;
    pub mod route;
    
    pub const CACHE_TTL: usize = 5 * 60;
}