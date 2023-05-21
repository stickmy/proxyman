use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)), context(suffix(Error)))]
pub enum ConfigurationErrorKind {
    Ssl { source: openssl::error::ErrorStack },
    Cert {},
    AppConf { source: std::io::Error },
    Unknown {},
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)), context(suffix(Error)))]
pub enum EndpointError {
    Io {
        source: std::io::Error,
    },
    Connect {
        source: hyper::Error,
    },
    Http {
        source: hyper::Error,
    },
    Uri {
        source: http::uri::InvalidUri,
    },
    UriParts {
        source: http::uri::InvalidUriParts,
    },
    WebsocketProtocol {
        source: hyper_tungstenite::tungstenite::error::ProtocolError,
    },
    #[snafu(display("Error occurred when decoding {}", scenario))]
    Decoder {
        scenario: &'static str,
    },
    #[snafu(display("{}", reason))]
    Proxyman {
        reason: &'static str,
    },
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)), context(suffix(Error)))]
pub enum Error {
    #[snafu(display("Configuration error: {}", reason))]
    Configuration {
        reason: &'static str,
        source: ConfigurationErrorKind,
    },
    #[snafu(display("Error occurred with the server in {}: {}", scenario, source))]
    Server {
        scenario: &'static str,
        source: EndpointError,
    },
    #[snafu(display("Error occurred with the client in {}: {}", scenario, source))]
    Client {
        scenario: &'static str,
        source: EndpointError,
    },
}
