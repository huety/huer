use std::collections::HashSet;

use reqwest::{Client, Url};
use url::Host;

use crate::error::Error;

/// The core component
///
/// The Philips Hue Bridge is a ZigBee hub used to talk with lights and other
/// devices. It offers a [REST API][^1] which is used by this crate.
///
/// [REST API]: <https://developers.meethue.com/develop/hue-api-v2/api-reference/>
///
/// [^1]: Most of the documentation (like the [REST API] reference) is only
/// accessible with a [Hue developer account].
///
/// [Hue developer account]: <https://developers.meethue.com/login/>
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Bridge {
    /// The host name where the HTTP requests are sent to.
    host: Host,
    /// The unique id of the bridge. The `host` may change but the `id` doesn't.
    id: String,
    /// The port for the REST API. Note that [Hue entertainment] uses a
    /// different port.
    ///
    /// [Hue entertainment]: <https://developers.meethue.com/develop/hue-entertainment/hue-entertainment-api/>
    port: u16,
}

impl Bridge {
    /// The [`Host`] which is used for the communication with the [Bridge].
    ///
    /// Note that the [Host] should not be used for the unique identification
    /// of a [Bridge]. Instead, use the `id`.
    pub fn host(&self) -> &Host {
        &self.host
    }

    /// The unique, permanent identifier of the [Bridge]. Unlike the [Host],
    /// this won't change.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The port at which the REST API is located. This should be 443 for https
    /// or 80 for http.
    pub fn port(&self) -> u16 {
        self.port
    }

    /*/// Returns the base for the clip API v2: `https://{bridge address}/clip/v2`
    pub(crate) fn base(&self) -> Url {
        let mut base = Url::parse("/clip/v2").unwrap();
        base.set_host(Some(&self.host.to_string())).unwrap();
        // bridge uses tls with a cert signed by signify / philips hues root ca
        base.set_scheme("https").unwrap();
        base
    }*/
    /// Bridge discovery
    ///
    /// This function tries to discover [`Bridge`]s on the same network.
    ///
    /// If the `discover_mdns` feature flag is active, it will try to find the
    /// bridge via mdns.
    ///
    /// If the `discover_remote` feature flag is active, it will try to make a
    /// http request to [Philips Hue's discovery] endpoint. For the http
    /// request, it will use the `client`.
    ///
    /// If both feature flags are active, it will first search using mdns and
    /// then using the discovery endpoint.
    ///
    /// For further information, see the [documentation] in the developer
    /// portal.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// use huer::Bridge;
    ///
    /// // For the discovery endpoint
    /// let client = reqwest::Client::new();
    ///
    /// let bridges = Bridge::discover(&client).await.unwrap();
    /// println!("Bridges found: {}", bridges.count());
    /// # });
    /// ```
    /// [documentation]: <https://developers.meethue.com/develop/application-design-guidance/hue-bridge-discovery/>
    ///
    /// [Philips Hue's discovery]: <https://discovery.meethue.com/>
    pub async fn discover(client: &Client) -> Result<impl Iterator<Item = Self>, Error> {
        let mut discovered = HashSet::new();

        #[cfg(feature = "discover_remote")]
        {
            use reqwest::{Method, Request};
            let endpoint = Url::parse("https://discovery.meethue.com/").unwrap();
            let request = Request::new(Method::GET, endpoint);

            // parse them and add them to the already discovered bridges
            discovered.extend(
                client
                    .execute(request)
                    .await?
                    .json::<HashSet<discover_remote::Response>>()
                    .await?
                    .into_iter()
                    .map(Into::into),
            );
        }

        // TODO: add mDNS
        Ok(discovered.into_iter())
    }
}

#[cfg(feature = "discover_remote")]
mod discover_remote {
    use serde::{de::Error, Deserialize, Deserializer};
    use url::Host;

    use crate::Bridge;
    #[derive(serde::Deserialize, PartialEq, Eq, Hash)]
    pub(super) struct Response {
        #[serde(rename = "internalipaddress")]
        #[serde(deserialize_with = "deserialize_host")]
        ip: Host,
        id: String,
        port: u16,
    }

    /// serde implementations for [`Host`] are not as expected.
    /// See <https://github.com/servo/rust-url/issues/543>
    ///
    /// This function deserializes a [`Host`] from a string
    fn deserialize_host<'de, D>(d: D) -> Result<Host, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str as Deserialize>::deserialize(d)?;
        Host::parse(s).map_err(D::Error::custom)
    }
    impl From<Response> for Bridge {
        fn from(r: Response) -> Self {
            Self {
                host: r.ip,
                id: r.id,
                port: r.port,
            }
        }
    }
}
