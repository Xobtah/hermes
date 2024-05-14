use axum::{response::IntoResponse, routing::get, Json, Router};
use base64::{prelude::BASE64_STANDARD, Engine as _};
use common::{api, crypto};
use log::info;

const IDENTITY: &str = "R3/UgL+tpWI7OM44Q6JdgOyZ3WiZnvS30KRXlUzHWrU=";

async fn index(body: Json<api::Registration>) -> impl IntoResponse {
    let mut signing_key = crypto::get_identity_from(
        BASE64_STANDARD
            .decode(IDENTITY)
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap(),
    )
    .unwrap();
    crypto::verify_key_exchange_key_pair(&body.identity, body.public_key, body.signature).unwrap();
    let (public_key, nonce, encrypted_data) =
        crypto::encrypt(body.public_key, ":)".as_bytes()).unwrap();
    let signature =
        crypto::sign(&mut signing_key, &[], public_key, &encrypted_data, nonce).unwrap();
    Json(api::Response {
        public_key,
        nonce,
        encrypted_data,
        signature,
    })
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Hello, world!");
    // build our application with a single route
    // let app = Router::new().route("/", get(|| async { "Hello, World!" }));
    let app = Router::new().route("/", get(index));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
