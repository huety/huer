use std::time::Duration;

use reqwest::{Client, Url};
use secrecy::SecretString;
use serde::Deserialize;
use serde_json::json;
use tokio::{
    select,
    time::{sleep, sleep_until, Instant},
};

use crate::{
    error::{AuthenticationError, Result},
    Bridge,
};

#[derive(Debug, Clone)]
#[non_exhaustive]
/// Represents an authenticated device/app interacting with the bridge.
/// The bridge physically authenticates devices using a button press.
pub struct Authenticator {
    username: SecretString,
    clientkey: SecretString,
}

impl Authenticator {
    /// The `username` used to authenticate with the [Bridge]
    pub fn username(&self) -> &SecretString {
        &self.username
    }

    /// The `clientkey` used to authenticate with the [Bridge]
    pub fn clientkey(&self) -> &SecretString {
        &self.clientkey
    }
}

#[derive(Debug, Deserialize)]
enum Response {
    #[serde(rename = "success")]
    Ok(Success),
    #[serde(rename = "error")]
    Err(Failure),
}

#[derive(Debug, Deserialize)]
struct Success {
    username: SecretString,
    clientkey: SecretString,
}

#[derive(Debug, Deserialize)]
struct Failure {
    #[serde(rename = "type")]
    typ: u16,
}

impl Authenticator {
    /// This function keeps asking the [`Bridge`] for an [`Authenticator`] until
    /// `deadline` is reached. It will start with the first
    /// request and will then, if not successful, ask after `poll_rate` time again.
    ///
    /// # Example
    ///
    /// ```no_run # needs a bridge on the local network to run
    /// # tokio_test::block_on(async {
    /// use huer::{Authenticator, Bridge};
    /// use tokio::time::Instant;
    /// use std::time::Duration;
    /// use reqwest::{ClientBuilder, tls::Certificate};
    ///
    /// let root_ca = Certificate::from_pem(huer::HUE_BRIDGE_ROOT_CA).unwrap();
    /// let client = ClientBuilder::new().danger_accept_invalid_certs(true).build().unwrap();
    ///
    ///
    /// let bridge = Bridge::discover(&client).await.unwrap().next().unwrap();
    ///
    /// let deadline = Instant::now() + Duration::from_secs(30);
    /// let authenticator = Authenticator::request(
    ///     &bridge,
    ///     &client,
    ///     "my_app",
    ///     deadline,
    ///     Duration::from_secs(1),
    ///     
    /// ).await;
    /// # });
    /// ```
    pub async fn request(
        bridge: &Bridge,
        client: &Client,
        device_type: &str,
        deadline: Instant,
        poll_rate: Duration,
    ) -> Result<Self> {
        select! {
            result = async {
                loop {
                    let payload = json!({
                        "devicetype": device_type,
                        "generateclientkey": true,
                    });

                    // authentication is not built into the clip v2 api yet so we cant use `Bridge::base()`
                    let url = Url::parse(&format!("https://{}/api", bridge.host()))
                        .unwrap();

                    let [res]: [Response; 1] = client
                    .post(url)
                    .json(&payload)
                    .send()
                    .await?
                    .json()
                    .await?;
                    match res {
                        Response::Ok(s) => return Ok(Self {
                            clientkey: s.clientkey,
                            username:s.username
                        }),
                        Response::Err(f) => {
                            if f.typ != 101 {
                                return Err(AuthenticationError::Other(f.typ).into());
                            }
                        },
                    };

                    sleep(poll_rate).await;
                }
            } => result,
            _ = sleep_until(deadline) => Err(AuthenticationError::TimedOut.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Response;

    #[test]
    fn deserialize_err_response() {
        let err_res =
            r#"[{"error":{"type":101,"address":"","description":"link button not pressed"}}]"#;
        let e: [Response; 1] = serde_json::from_str(err_res).unwrap();
        assert!(matches!(e[0], Response::Err(_)));
    }

    #[test]
    fn deserialize_ok_response() {
        let ok_res = r#"[{"success":{"username":"TEzEX5HApPsYDwTmI2HqBHL-4MuQRYf8SmMB-4Wv","clientkey":"C2B605F53DCA6814775A2C9ACB20F3C3"}}]"#;
        let o: [Response; 1] = serde_json::from_str(ok_res).unwrap();
        assert!(matches!(o[0], Response::Ok(_)));
    }
}
