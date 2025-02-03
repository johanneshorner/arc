use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

#[derive(Serialize, Debug)]
pub struct LoginRequest {
    #[serde(rename = "userName")]
    pub user_name: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
struct LoginResponse {
    cookie: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct PortPoe {
    pub uri: String,
    pub port_id: String,
    pub is_poe_enabled: bool,
    pub poe_priority: String,
    pub poe_allocation_method: String,
    pub allocated_power_in_watts: u32,
    pub port_configured_type: String,
    pub pre_standard_detect_enabled: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct PortPoeWrite {
    is_poe_enabled: bool,
}

impl From<PortPoe> for PortPoeWrite {
    fn from(port_poe: PortPoe) -> Self {
        Self {
            is_poe_enabled: port_poe.is_poe_enabled,
        }
    }
}

impl From<&PortPoe> for PortPoeWrite {
    fn from(port_poe: &PortPoe) -> Self {
        Self {
            is_poe_enabled: port_poe.is_poe_enabled,
        }
    }
}

#[derive(Deserialize, Debug)]
struct WiredElementList {
    #[allow(dead_code)]
    collection_result: HashMap<String, u32>,
    port_poe: Vec<PortPoe>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not parse url: {0}")]
    Parse(url::ParseError),
    #[error("Could not send request: {0}")]
    Request(reqwest::Error),
}

impl From<url::ParseError> for Error {
    fn from(value: url::ParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::Request(value)
    }
}

#[derive(Clone, Debug)]
pub struct Session {
    client: reqwest::Client,
    url: reqwest::Url,
}

impl Session {
    pub async fn new(url: &str, credentials: &LoginRequest) -> Result<(Session, String), Error> {
        let rest_base_url = reqwest::Url::parse(url)?.join("rest/v1/")?;

        let client = reqwest::ClientBuilder::new().cookie_store(true).build()?;

        let LoginResponse { cookie } = client
            .post(rest_base_url.join("login-sessions")?)
            .json(credentials)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok((Session::from_cookie(url, &cookie)?, cookie))
    }

    pub fn from_cookie(url: &str, session_cookie: &str) -> Result<Session, Error> {
        let rest_base_url = reqwest::Url::parse(url)?.join("rest/v1/")?;

        let cookies = reqwest::cookie::Jar::default();
        cookies.add_cookie_str(session_cookie, &reqwest::Url::parse(url)?);

        let client = reqwest::ClientBuilder::new()
            .cookie_provider(Arc::new(cookies))
            .build()?;

        Ok(Session {
            client,
            url: rest_base_url,
        })

        // TODO perform some request to check if the session_cookie is still valid
    }

    pub async fn get_ports(&self) -> Result<Vec<PortPoe>, Error> {
        let url = self.url.join("poe/ports")?;
        let response = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<WiredElementList>()
            .await?;
        Ok(response.port_poe)
    }

    pub async fn get_port<T>(&self, port_id: T) -> Result<PortPoe, Error>
    where
        T: AsRef<str> + std::fmt::Display,
    {
        let url = self.url.join(&format!("ports/{port_id}/poe"))?;
        Ok(self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<PortPoe>()
            .await?)
    }

    pub async fn set_port(
        &self,
        port: &PortPoe,
        data: &serde_json::Value,
    ) -> Result<PortPoe, Error> {
        let url = self.url.join(&format!("ports/{}/poe", port.port_id))?;
        Ok(self
            .client
            .put(url)
            .json(data)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}
