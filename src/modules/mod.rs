pub const CACHE_TTL: usize = 5 * 60;

pub mod user {
    pub mod handle;
    pub mod model;
    pub mod repository;
    pub mod repository_pg;
    pub mod route;
    pub mod schema;
    pub mod service;
}

pub mod friend {
    pub mod handle;
    pub mod model;
    pub mod repository;
    pub mod repository_pg;
    pub mod route;
    pub mod schema;
    pub mod service;
}

#[allow(unused)]
pub mod message {
    pub mod handle;
    pub mod model;
    pub mod repository;
    pub mod repository_pg;
    pub mod route;
    pub mod schema;
    pub mod service;
}

pub mod conversation {
    pub mod handle;
    pub mod model;
    pub mod repository;
    pub mod repository_pg;
    pub mod route;
    pub mod schema;
    pub mod service;
}
