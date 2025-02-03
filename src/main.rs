mod api;

use crate::api::LoginRequest;
use anyhow::{anyhow, bail};
use api::{PortPoe, Session};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    commands: Command,
}

#[derive(Subcommand)]
enum Command {
    Login(LoginArgs),
    #[command(subcommand)]
    Port(PortCommands),
    Completion {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Serialize, Deserialize, clap::Args, Clone, Debug)]
struct LoginArgs {
    base_url: String,
    user_name: String,
    password: String,
}

impl From<LoginArgs> for LoginRequest {
    fn from(value: LoginArgs) -> Self {
        LoginRequest {
            user_name: value.user_name,
            password: value.password,
        }
    }
}

impl From<&LoginArgs> for LoginRequest {
    fn from(value: &LoginArgs) -> Self {
        LoginRequest {
            user_name: value.user_name.clone(),
            password: value.password.clone(),
        }
    }
}

impl LoginArgs {
    async fn handle(&self) -> anyhow::Result<Session> {
        let (session, cookie) = Session::new(&self.base_url, &self.into()).await?;

        PersistentData::save_to_disk(self.clone(), cookie).await?;

        Ok(session)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PersistentData {
    login_args: LoginArgs,
    cookie: String,
}

impl PersistentData {
    async fn save_to_disk(login_args: LoginArgs, cookie: String) -> anyhow::Result<()> {
        if let Some(data_dir) = dirs::data_dir() {
            let arc_dir = data_dir.join("arc");
            if !fs::try_exists(&arc_dir).await? {
                fs::create_dir(&arc_dir).await?;
            }
            let data_to_persist = PersistentData { login_args, cookie };
            let serialized = serde_json::to_string(&data_to_persist)?;

            fs::write(arc_dir.join("persist.txt"), serialized).await?;
            Ok(())
        } else {
            bail!("data directory does not exist");
        }
    }

    async fn load_from_disk() -> anyhow::Result<PersistentData> {
        if let Some(data_dir) = dirs::data_dir() {
            let data_as_json = fs::read_to_string(data_dir.join("arc/persist.txt")).await?;
            Ok(serde_json::from_str(&data_as_json)?)
        } else {
            bail!("data directory does not exist");
        }
    }
}

#[derive(Subcommand)]
enum PortCommands {
    Get(GetArgs),
    Set(SetArgs),
}

#[derive(clap::Args)]
struct GetArgs {
    port_ids: Vec<String>,
}

impl GetArgs {
    async fn handle(self, session: Session) -> anyhow::Result<()> {
        let all = self.port_ids.len() == 1 && self.port_ids[0] == "all";

        let ports = if all {
            session.get_ports().await?
        } else if self.port_ids.len() == 1 {
            vec![session.get_port(&self.port_ids[0]).await?]
        } else {
            session
                .get_ports()
                .await?
                .into_iter()
                .filter(|port| self.port_ids.contains(&port.port_id))
                .collect()
        };

        for port in ports {
            println!("{}", serde_json::to_string(&port)?);
        }

        Ok(())
    }
}

#[derive(clap::Args)]
struct SetArgs {
    #[arg(required(true))]
    port_ids: Vec<String>,
    data: String,
}

impl SetArgs {
    async fn handle(self, session: Session) -> anyhow::Result<()> {
        let all = self.port_ids.len() == 1 && self.port_ids[0] == "all";

        let ports = if all {
            session.get_ports().await?
        } else {
            session
                .get_ports()
                .await?
                .into_iter()
                .filter(|port| self.port_ids.contains(&port.port_id))
                .collect()
        };

        let mut tasks = Vec::with_capacity(ports.len());
        for port in ports {
            let session_clone = session.clone();
            let json_data = serde_json::from_str(&self.data)?;
            tasks.push(tokio::spawn(async move {
                session_clone.set_port(&port, &json_data).await
            }));
        }

        let results = futures::future::join_all(tasks)
            .await
            .into_iter()
            .flat_map(|task| {
                task.map_err(|e| {
                    Err::<Result<Vec<PortPoe>, api::Error>, anyhow::Error>(anyhow!(
                        "failed to join task: {e}"
                    ))
                })
            })
            .collect::<Result<Vec<PortPoe>, api::Error>>()?;

        for result in results {
            println!("{}", serde_json::to_string(&result)?);
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Args::parse();

    match cli.commands {
        Command::Login(args) => _ = args.handle().await?,
        Command::Port(port_command) => {
            let persisted_data = PersistentData::load_from_disk()
                .await
                .map_err(|e| anyhow!("couldn't load cookie from disk, please login first: {e}"))?;

            let session =
                Session::from_cookie(&persisted_data.login_args.base_url, &persisted_data.cookie)?;

            let session = if let Err(api::Error::Request(e)) = session.get_port("1").await {
                if let Some(status_code) = e.status() {
                    if status_code.as_u16() == 400 {
                        let login_request: LoginRequest = persisted_data.login_args.clone().into();
                        let (new_session, cookie) =
                            Session::new(&persisted_data.login_args.base_url, &login_request)
                                .await
                                .map_err(|e| anyhow!("couldn't create session: {e}"))?;

                        PersistentData::save_to_disk(persisted_data.login_args, cookie).await?;

                        new_session
                    } else {
                        bail!("unexpected error");
                    }
                } else {
                    bail!("unexpected error");
                }
            } else {
                session
            };

            match port_command {
                PortCommands::Get(args) => args.handle(session).await?,
                PortCommands::Set(args) => args.handle(session).await?,
            }
        }
        Command::Completion { shell } => {
            clap_complete::generate(shell, &mut Args::command(), "arc", &mut std::io::stdout())
        }
    }

    Ok(())
}
