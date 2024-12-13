use async_once::AsyncOnce;
use lazy_static::lazy_static;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use tracing::{event, Level};

lazy_static! {
    pub static ref DB: AsyncOnce<Surreal<Client>> = {
        AsyncOnce::new(async {
            // load env vars
            let database_url = std::env::var("DATABASE_URL").unwrap_or("127.0.0.1:8000".to_string());
            let username = std::env::var("DATABASE_USERNAME").unwrap_or("root".to_string());
            let password = std::env::var("DATABASE_PASSWORD").unwrap_or("root".to_string());
            event!(Level::INFO, "Connecting to database at {}", database_url);
            event!(Level::INFO, "Using username {}", username);
            let db: Surreal<Client> = Surreal::new::<Ws>(database_url)
                .await
                .expect("couldn't connect to surrealdb");

            db.signin(Root {
                username: &*username,
                password: &*password,
            })
            .await
            .expect("couldn't sign in");

            db.use_ns("wordweaver")
                .use_db("wordweaver")
                .await
                .expect("could not use ns and db");

            db
        })
    };
}