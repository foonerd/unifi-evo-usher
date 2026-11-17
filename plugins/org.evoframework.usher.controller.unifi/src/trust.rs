//! Whom the plugin believes it is speaking to. A controller is believed
//! on a certificate the public authorities vouch for, or, where the
//! operator has said so, on one certificate held to its fingerprint: a
//! console's own certificate is signed by nobody.

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use rustls::crypto::{
    verify_tls12_signature, verify_tls13_signature, CryptoProvider,
};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    ClientConfig, DigitallySignedStruct, Error, RootCertStore, SignatureScheme,
};
use sha2::{Digest, Sha256};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;

/// How long a controller is given to open a connection and shake hands.
const HANDSHAKE: Duration = Duration::from_secs(5);

/// The SHA-256 of a certificate as it is sent, which is what an operator
/// compares.
pub type Fingerprint = [u8; 32];

/// The fingerprint of a certificate.
pub fn fingerprint_of(certificate: &CertificateDer<'_>) -> Fingerprint {
    Sha256::digest(certificate.as_ref()).into()
}

/// A fingerprint as an operator reads and types it: 64 hexadecimal
/// digits, in either case, with or without colons or spaces between them.
pub fn fingerprint_read(text: &str) -> Option<Fingerprint> {
    let digits: String = text
        .chars()
        .filter(|c| !matches!(c, ':' | ' ' | '-'))
        .collect();
    hex::decode(digits).ok()?.try_into().ok()
}

/// A fingerprint written out: 64 hexadecimal digits in lower case.
pub fn fingerprint_written(fingerprint: &Fingerprint) -> String {
    hex::encode(fingerprint)
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Believes exactly one certificate, whoever signed it and whatever name
/// it carries, and no other.
#[derive(Debug)]
struct Held {
    fingerprint: Fingerprint,
    provider: Arc<CryptoProvider>,
}

/// Believes any certificate and remembers which one it was shown. Used
/// only to tell an operator what a controller presents; nothing is sent
/// over a connection made with it.
#[derive(Debug)]
struct Witness {
    seen: Mutex<Option<Fingerprint>>,
    provider: Arc<CryptoProvider>,
}

macro_rules! signatures_by {
    ($provider:ident) => {
        fn verify_tls12_signature(
            &self,
            message: &[u8],
            certificate: &CertificateDer<'_>,
            signed: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            verify_tls12_signature(
                message,
                certificate,
                signed,
                &self.$provider.signature_verification_algorithms,
            )
        }

        fn verify_tls13_signature(
            &self,
            message: &[u8],
            certificate: &CertificateDer<'_>,
            signed: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            verify_tls13_signature(
                message,
                certificate,
                signed,
                &self.$provider.signature_verification_algorithms,
            )
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            self.$provider
                .signature_verification_algorithms
                .supported_schemes()
        }
    };
}

impl ServerCertVerifier for Held {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        if fingerprint_of(end_entity) == self.fingerprint {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(Error::General(
                "the controller presents another certificate than the one \
                 it is held to"
                    .to_string(),
            ))
        }
    }

    signatures_by!(provider);
}

impl ServerCertVerifier for Witness {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        *self.seen.lock().expect("the witness's lock") =
            Some(fingerprint_of(end_entity));
        Ok(ServerCertVerified::assertion())
    }

    signatures_by!(provider);
}

/// The rules a connection to the controller is made under: the one
/// certificate it is held to, or the public authorities where it is held
/// to none.
pub fn rules(held: Option<Fingerprint>) -> Result<ClientConfig, Error> {
    let provider = provider();
    let rules = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()?;
    Ok(match held {
        Some(fingerprint) => rules
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(Held {
                fingerprint,
                provider,
            }))
            .with_no_client_auth(),
        None => {
            let mut roots = RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            rules.with_root_certificates(roots).with_no_client_auth()
        }
    })
}

/// What a controller presents, for an operator to look at before the
/// controller is believed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presented {
    /// The fingerprint of the certificate it presents.
    pub fingerprint: Fingerprint,
    /// Whether the public authorities vouch for that certificate under
    /// the name the controller was reached by.
    pub vouched: bool,
}

async fn shake(host: &str, port: u16, rules: ClientConfig) -> io::Result<()> {
    let name = ServerName::try_from(host.to_string())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let shaken = async {
        let line = TcpStream::connect((host, port)).await?;
        TlsConnector::from(Arc::new(rules))
            .connect(name, line)
            .await
            .map(drop)
    };
    timeout(HANDSHAKE, shaken)
        .await
        .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))?
}

/// Shake hands with a controller and say what it presents. Nothing is
/// sent to it: no request, and no key.
pub async fn presented(host: &str, port: u16) -> io::Result<Presented> {
    let provider = provider();
    let witness = Arc::new(Witness {
        seen: Mutex::new(None),
        provider: provider.clone(),
    });
    let rules_of_witness = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(io::Error::other)?
        .dangerous()
        .with_custom_certificate_verifier(witness.clone())
        .with_no_client_auth();
    shake(host, port, rules_of_witness).await?;
    let fingerprint =
        witness
            .seen
            .lock()
            .expect("the witness's lock")
            .take()
            .ok_or_else(|| io::Error::other("no certificate was presented"))?;
    let vouched = match rules(None) {
        Ok(public) => shake(host, port, public).await.is_ok(),
        Err(_) => false,
    };
    Ok(Presented {
        fingerprint,
        vouched,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fingerprint_is_read_as_an_operator_types_it() {
        let plain = "ab".repeat(32);
        let spaced = plain
            .as_bytes()
            .chunks(2)
            .map(|pair| std::str::from_utf8(pair).unwrap().to_uppercase())
            .collect::<Vec<_>>()
            .join(":");
        let read = fingerprint_read(&spaced).unwrap();
        assert_eq!(read, [0xab; 32]);
        assert_eq!(fingerprint_written(&read), plain);
        assert_eq!(fingerprint_read("abcd"), None, "too short");
        assert_eq!(fingerprint_read(&"zz".repeat(32)), None, "not digits");
    }
}
