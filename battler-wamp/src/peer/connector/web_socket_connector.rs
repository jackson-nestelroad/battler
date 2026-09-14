use std::sync::Arc;

use anyhow::{
    Context,
    Error,
    Result,
};
use async_trait::async_trait;
use log::warn;
use rustls::pki_types::{
    CertificateDer,
    PrivateKeyDer,
    pem::PemObject,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    Connector as RustlsConnector,
    MaybeTlsStream,
    WebSocketStream,
    connect_async_tls_with_config,
    tungstenite::{
        ClientRequestBuilder,
        http::{
            Uri as HttpUri,
            header::SEC_WEBSOCKET_PROTOCOL,
        },
    },
};

use crate::{
    peer::{
        connector::connector::{
            Connection,
            Connector,
            ConnectorFactory,
        },
        peer::{
            ClientMutualTlsPaths,
            PeerConfig,
        },
    },
    serializer::serializer::SerializerType,
};

#[derive(Default)]
struct WebSocketConnector {}

#[async_trait]
impl Connector<WebSocketStream<MaybeTlsStream<TcpStream>>> for WebSocketConnector {
    async fn connect(
        &self,
        config: &PeerConfig,
        uri: &str,
    ) -> Result<Connection<WebSocketStream<MaybeTlsStream<TcpStream>>>> {
        let request_uri: HttpUri = uri.try_into()?;
        let is_wss = request_uri.scheme_str() == Some("wss");
        let mut request = ClientRequestBuilder::new(request_uri);
        if !config.agent.is_empty() {
            request = request.with_header("User-Agent", &config.agent);
        }
        for protocol in &config.serializers {
            request = request.with_sub_protocol(protocol.uri().to_string());
        }

        let mut rustls_connector = None;
        if let Some(web_socket) = &config.web_socket {
            for (key, value) in &web_socket.headers {
                request = request.with_header(key, value);
            }
            if let Some(mutual_tls) = &web_socket.mutual_tls {
                if is_wss {
                    rustls_connector = Some(RustlsConnector::Rustls(build_rustls_client_config(
                        mutual_tls,
                    )?));
                } else {
                    warn!(
                        "peer {} is configured for mutual TLS, but {uri} is not a wss URI; the \
                         connection will not be encrypted",
                        config.name
                    );
                }
            }
        }

        let (stream, response) =
            connect_async_tls_with_config(request, None, false, rustls_connector).await?;
        let serializer = match response.headers().get(SEC_WEBSOCKET_PROTOCOL) {
            Some(protocol) => {
                let protocol = protocol.to_str()?;
                SerializerType::try_from(protocol).map_err(Error::msg)?
            }
            None => return Err(Error::msg("handshake did not produce a sub-protocol")),
        };

        Ok(Connection { stream, serializer })
    }
}

/// A factory for generating [`Connector`]s for WebSocket connections.
#[derive(Default)]
pub struct WebSocketConnectorFactory {}

impl ConnectorFactory<WebSocketStream<MaybeTlsStream<TcpStream>>> for WebSocketConnectorFactory {
    fn new_connector(
        &self,
    ) -> Box<dyn Connector<WebSocketStream<MaybeTlsStream<TcpStream>>> + Send> {
        Box::new(WebSocketConnector::default())
    }
}

// Builds a rustls client configuration for mutual TLS from PEM files on disk.
fn build_rustls_client_config(
    mutual_tls: &ClientMutualTlsPaths,
) -> Result<Arc<rustls::ClientConfig>> {
    let mut root_cert_store = rustls::RootCertStore::empty();
    for cert in CertificateDer::pem_file_iter(&mutual_tls.ca_cert_path)
        .context("failed to read CA certificate file")?
    {
        root_cert_store.add(cert.context("failed to parse CA certificate")?)?;
    }

    let client_certs = CertificateDer::pem_file_iter(&mutual_tls.client_cert_path)
        .context("failed to read client certificate file")?
        .collect::<Result<Vec<_>, _>>()
        .context("failed to parse client certificate")?;
    let client_key = PrivateKeyDer::from_pem_file(&mutual_tls.client_key_path)
        .context("failed to read client private key file")?;

    Ok(Arc::new(
        rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_client_auth_cert(client_certs, client_key)?,
    ))
}

#[cfg(test)]
mod tls_test {
    use crate::peer::{
        connector::web_socket_connector::build_rustls_client_config,
        peer::ClientMutualTlsPaths,
    };

    #[test]
    fn build_rustls_client_config_fails_for_missing_ca_cert() {
        assert_matches::assert_matches!(
            build_rustls_client_config(&ClientMutualTlsPaths {
                ca_cert_path: "/bogus/ca.pem".to_owned(),
                client_cert_path: "/bogus/client.pem".to_owned(),
                client_key_path: "/bogus/client-key.pem".to_owned(),
            }),
            Err(err) => {
                assert!(err.to_string().contains("failed to read CA certificate file"));
            }
        );
    }
}
