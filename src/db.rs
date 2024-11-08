use async_once::AsyncOnce;
use lazy_static::lazy_static;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

lazy_static! {
    pub static ref DB: AsyncOnce<Surreal<Client>> = {
        AsyncOnce::new(async {
            let db: Surreal<Client> = Surreal::new::<Ws>("127.0.0.1:8000")
                .await
                .expect("couldn't connect to surrealdb");

            db.signin(Root {
                username: "root",
                password: "root",
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

// #[derive(Debug, Serialize, Deserialize)]
// pub struct GameDB<'a> {
//     pub name: &'a String,
//     pub gaps: Vec<String>,
// }