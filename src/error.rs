use reqwest::Error as HttpError;

/// Shortcut for [Result]s that return [Error]
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
/// Typed Errors that may occur while using this crate
pub enum Error {
    #[error(transparent)]
    /// An error connected with the HTTP connection to the hue bridge
    Http(#[from] HttpError),
    #[error(transparent)]
    /// Errors that may happen during authentication
    Authentication(#[from] AuthenticationError),
}

#[derive(Debug, thiserror::Error)]
/// This error is returned by [`crate::Authenticator`] during authentication.
pub enum AuthenticationError {
    #[error("reached the request deadline")]
    /// It took the user too long to press the button on the hue bridge
    TimedOut,
    #[error("api returned error code {0}")]
    /// API error code `101` is catched by the [`crate::Authenticator`], but others aren't
    /// since them are from CLIPv1.
    /// For possible error codes, see <https://developers.meethue.com/develop/hue-api/error-messages/>
    Other(u16),
}
