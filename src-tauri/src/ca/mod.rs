use std::{
    fs,
    io::Write,
    net::IpAddr,
    path::Path,
    sync::Arc,
};
use std::time::{Duration, SystemTime};

use crate::error::configuration_error::{AppConfIoError, SslError};
use crate::error::{self, *};
use async_trait::async_trait;
use http::uri::Authority;
use moka::future::Cache;
use openssl::{
    asn1::{Asn1Integer, Asn1Time},
    bn::BigNum,
    hash::MessageDigest,
    pkey::{PKey, Private},
    rand,
    rsa::Rsa,
    x509::{
        extension::{BasicConstraints, KeyUsage, SubjectAlternativeName, SubjectKeyIdentifier},
        X509Builder, X509NameBuilder, X509,
    },
};
use snafu::ResultExt;
use tokio_rustls::rustls::{self, ServerConfig};

const TTL_SECS: i64 = 365 * 24 * 60 * 60;
const CACHE_TTL: u64 = TTL_SECS as u64 / 2;
const NOT_BEFORE_OFFSET: i64 = 60;
const ROOT_CA_CN: &str = "Proxyman Local Root CA";

#[async_trait]
pub trait CertificateAuthority: Send + Sync + 'static {
    async fn gen_server_config(&self, authority: &Authority) -> Arc<ServerConfig>;
}

#[derive(Clone)]
pub struct Ssl {
    ca_pkey: PKey<Private>,
    leaf_pkey: PKey<Private>,
    private_key: rustls::PrivateKey,
    ca_cert: X509,
    hash: MessageDigest,
    cache: Cache<Authority, Arc<ServerConfig>>,
}

impl Default for Ssl {
    fn default() -> Self {
        let material = RootCaMaterial::generate().expect("Failed to generate root CA");
        Self::from_material(material).expect("Failed to build SSL CA")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RootCaMaterial {
    certificate_pem: Vec<u8>,
    private_key_pem: Vec<u8>,
}

impl RootCaMaterial {
    fn generate() -> Result<Self, error::Error> {
        let ca_pkey = PKey::from_rsa(
            Rsa::generate(2048)
                .context(SslError {})
                .context(ConfigurationError {
                    scenario: "Generate root CA private key",
                })?,
        )
        .context(SslError {})
        .context(ConfigurationError {
            scenario: "Create root CA private key",
        })?;

        let mut name_builder = X509NameBuilder::new()
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Create root CA name",
            })?;
        name_builder
            .append_entry_by_text("CN", ROOT_CA_CN)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Set root CA common name",
            })?;
        let name = name_builder.build();

        let mut x509_builder = X509Builder::new()
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Create root CA certificate",
            })?;
        x509_builder
            .set_version(2)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Set root CA version",
            })?;
        let serial_number = random_serial_number()?;
        x509_builder
            .set_serial_number(&serial_number)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Set root CA serial number",
            })?;
        x509_builder
            .set_subject_name(&name)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Set root CA subject",
            })?;
        x509_builder
            .set_issuer_name(&name)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Set root CA issuer",
            })?;

        let not_before = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to determine current UNIX time")
            .as_secs() as i64
            - NOT_BEFORE_OFFSET;
        x509_builder
            .set_not_before(
                Asn1Time::from_unix(not_before)
                    .context(SslError {})
                    .context(ConfigurationError {
                        scenario: "Set root CA not before",
                    })?
                    .as_ref(),
            )
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Apply root CA not before",
            })?;
        x509_builder
            .set_not_after(
                Asn1Time::from_unix(not_before + TTL_SECS)
                    .context(SslError {})
                    .context(ConfigurationError {
                        scenario: "Set root CA not after",
                    })?
                    .as_ref(),
            )
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Apply root CA not after",
            })?;
        x509_builder
            .set_pubkey(&ca_pkey)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Set root CA public key",
            })?;

        let basic_constraints = BasicConstraints::new()
            .critical()
            .ca()
            .build()
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Create root CA basic constraints",
            })?;
        x509_builder
            .append_extension(basic_constraints)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Append root CA basic constraints",
            })?;

        let key_usage = KeyUsage::new()
            .critical()
            .key_cert_sign()
            .crl_sign()
            .build()
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Create root CA key usage",
            })?;
        x509_builder
            .append_extension(key_usage)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Append root CA key usage",
            })?;

        let subject_key_identifier = SubjectKeyIdentifier::new()
            .build(&x509_builder.x509v3_context(None, None))
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Create root CA subject key identifier",
            })?;
        x509_builder
            .append_extension(subject_key_identifier)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Append root CA subject key identifier",
            })?;

        x509_builder
            .sign(&ca_pkey, MessageDigest::sha256())
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Sign root CA certificate",
            })?;

        let certificate = x509_builder.build();

        Ok(Self {
            certificate_pem: certificate
                .to_pem()
                .context(SslError {})
                .context(ConfigurationError {
                    scenario: "Encode root CA certificate",
                })?,
            private_key_pem: ca_pkey
                .private_key_to_pem_pkcs8()
                .context(SslError {})
                .context(ConfigurationError {
                    scenario: "Encode root CA private key",
                })?,
        })
    }

    pub(crate) fn load_or_generate<P: AsRef<Path>>(
        cert_path: P,
        key_path: P,
    ) -> Result<Self, error::Error> {
        let cert_path = cert_path.as_ref();
        let key_path = key_path.as_ref();

        if cert_path.exists() && key_path.exists() {
            return Ok(Self {
                certificate_pem: fs::read(cert_path)
                    .context(AppConfIoError {})
                    .context(ConfigurationError {
                        scenario: "Read root CA certificate",
                    })?,
                private_key_pem: fs::read(key_path)
                    .context(AppConfIoError {})
                    .context(ConfigurationError {
                        scenario: "Read root CA private key",
                    })?,
            });
        }

        if let Some(parent) = cert_path.parent() {
            fs::create_dir_all(parent)
                .context(AppConfIoError {})
                .context(ConfigurationError {
                    scenario: "Create root CA directory",
                })?;
        }
        if let Some(parent) = key_path.parent() {
            fs::create_dir_all(parent)
                .context(AppConfIoError {})
                .context(ConfigurationError {
                    scenario: "Create root CA key directory",
                })?;
        }

        let material = Self::generate()?;

        fs::write(cert_path, material.certificate_pem.as_slice())
            .context(AppConfIoError {})
            .context(ConfigurationError {
                scenario: "Write root CA certificate",
            })?;
        write_private_key(key_path, material.private_key_pem.as_slice())?;

        Ok(material)
    }
}

fn random_serial_number() -> Result<Asn1Integer, error::Error> {
    let mut serial_number = [0; 16];
    rand::rand_bytes(&mut serial_number)
        .context(SslError {})
        .context(ConfigurationError {
            scenario: "Create cryptographically strong random byte failed",
        })?;

    let serial_number = BigNum::from_slice(&serial_number)
        .context(SslError {})
        .context(ConfigurationError {
            scenario: "Create serial number failed",
        })?;

    Asn1Integer::from_bn(&serial_number)
        .context(SslError {})
        .context(ConfigurationError {
            scenario: "Create Asn1 integer failed",
        })
}

fn write_private_key(path: &Path, content: &[u8]) -> Result<(), error::Error> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(path).context(AppConfIoError {}).context(
        ConfigurationError {
            scenario: "Open root CA private key",
        },
    )?;
    file.write_all(content)
        .context(AppConfIoError {})
        .context(ConfigurationError {
            scenario: "Write root CA private key",
        })
}

impl Ssl {
    pub(crate) fn load_or_generate<P: AsRef<Path>>(
        cert_path: P,
        key_path: P,
    ) -> Result<Self, error::Error> {
        Self::from_material(RootCaMaterial::load_or_generate(cert_path, key_path)?)
    }

    fn from_material(material: RootCaMaterial) -> Result<Self, error::Error> {
        let ca_pkey = PKey::private_key_from_pem(material.private_key_pem.as_slice())
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Parse CA private key",
            })?;

        let ca_cert = X509::from_pem(material.certificate_pem.as_slice())
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Parse CA certificate",
            })?;

        let leaf_pkey = PKey::from_rsa(
            Rsa::generate(2048)
                .context(SslError {})
                .context(ConfigurationError {
                    scenario: "Generate leaf private key",
                })?,
        )
        .context(SslError {})
        .context(ConfigurationError {
            scenario: "Create leaf private key",
        })?;

        let private_key = rustls::PrivateKey(
            leaf_pkey
                .private_key_to_der()
                .context(SslError {})
                .context(ConfigurationError {
                    scenario: "Encode leaf private key",
                })?,
        );

        Ok(Self {
            ca_pkey,
            leaf_pkey,
            private_key,
            ca_cert,
            hash: MessageDigest::sha256(),
            cache: Cache::builder()
                .max_capacity(1_000)
                .time_to_live(Duration::from_secs(CACHE_TTL))
                .build(),
        })
    }

    fn gen_cert(&self, authority: &Authority) -> Result<rustls::Certificate, error::Error> {
        let mut name_builder =
            X509NameBuilder::new()
                .context(SslError {})
                .context(ConfigurationError {
                    scenario: "Create CA name builder failed",
                })?;
        name_builder
            .append_entry_by_text("CN", authority.host())
            .context(SslError)
            .context(ConfigurationError {
                scenario: "Append CA entry name failed",
            })?;
        let name = name_builder.build();

        let mut x509_builder =
            X509Builder::new()
                .context(SslError {})
                .context(ConfigurationError {
                    scenario: "Create X509 builder failed",
                })?;
        x509_builder
            .set_subject_name(&name)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Set CA subject name failed",
            })?;
        x509_builder
            .set_version(2)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Set version failed",
            })?;

        let not_before = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to determine current UNIX time")
            .as_secs() as i64
            - NOT_BEFORE_OFFSET;
        x509_builder
            .set_not_before(
                Asn1Time::from_unix(not_before)
                    .context(SslError {})
                    .context(ConfigurationError {
                        scenario: "Asn1 failed",
                    })?
                    .as_ref(),
            )
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "x509 not before error",
            })?;
        x509_builder
            .set_not_after(
                Asn1Time::from_unix(not_before + TTL_SECS)
                    .context(SslError {})
                    .context(ConfigurationError {
                        scenario: "Asn1 failed",
                    })?
                    .as_ref(),
            )
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "x509 not after error",
            })?;

        x509_builder
            .set_pubkey(&self.leaf_pkey)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Set pub key failed",
            })?;
        x509_builder
            .set_issuer_name(self.ca_cert.subject_name())
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Set issuer name failed",
            })?;

        let mut alternative_name = SubjectAlternativeName::new();
        if authority.host().parse::<IpAddr>().is_ok() {
            alternative_name.ip(authority.host());
        } else {
            alternative_name.dns(authority.host());
        }
        let alternative_name = alternative_name
            .build(&x509_builder.x509v3_context(Some(&self.ca_cert), None))
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Create x509 v3 context failed",
            })?;
        x509_builder
            .append_extension(alternative_name)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Append x509 extension failed",
            })?;

        let serial_number = random_serial_number()?;
        x509_builder
            .set_serial_number(&serial_number)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Set x509 serial number failed",
            })?;

        x509_builder
            .sign(&self.ca_pkey, self.hash)
            .context(SslError {})
            .context(ConfigurationError {
                scenario: "Sign x509 failed",
            })?;

        let x509 = x509_builder.build();

        Ok(rustls::Certificate(
            x509.to_der()
                .context(SslError {})
                .context(ConfigurationError {
                    scenario: "Transform x509 to der failed",
                })?,
        ))
    }
}

#[async_trait]
impl CertificateAuthority for Ssl {
    async fn gen_server_config(&self, authority: &Authority) -> Arc<ServerConfig> {
        if let Some(server_cfg) = self.cache.get(authority) {
            return server_cfg;
        }

        let certs = vec![self
            .gen_cert(authority)
            .unwrap_or_else(|_| panic!("Failed to generate certificate for {}", authority))];

        let mut server_cfg = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, self.private_key.clone())
            .expect("Failed to build ServerConfig");

        server_cfg.alpn_protocols = vec![
            #[cfg(feature = "http2")]
            b"h2".to_vec(),
            b"http/1.1".to_vec(),
        ];

        let server_cfg = Arc::new(server_cfg);

        self.cache
            .insert(authority.clone(), Arc::clone(&server_cfg))
            .await;

        server_cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn ca_generate_root_ca_creates_unique_private_keys() {
        let first = RootCaMaterial::generate().expect("failed to generate first CA");
        let second = RootCaMaterial::generate().expect("failed to generate second CA");

        assert_ne!(first.private_key_pem, second.private_key_pem);
        assert_ne!(first.certificate_pem, second.certificate_pem);
    }

    #[test]
    fn ca_load_or_generate_reuses_existing_root_material() {
        let dir = std::env::temp_dir().join(format!("proxyman-ca-{}", Uuid::new_v4()));
        let cert_path = dir.join("proxyman.cer");
        let key_path = dir.join("proxyman.key");

        let first = RootCaMaterial::load_or_generate(&cert_path, &key_path)
            .expect("failed to create CA material");
        let second = RootCaMaterial::load_or_generate(&cert_path, &key_path)
            .expect("failed to reload CA material");

        assert_eq!(first.private_key_pem, second.private_key_pem);
        assert_eq!(first.certificate_pem, second.certificate_pem);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ca_leaf_cert_contains_dns_san_for_hostname() {
        let material = RootCaMaterial::generate().expect("failed to generate CA");
        let ssl = Ssl::from_material(material).expect("failed to create ssl");
        let authority = Authority::from_static("example.com:443");

        let cert = ssl.gen_cert(&authority).expect("failed to generate cert");
        let x509 = X509::from_der(cert.0.as_slice()).expect("failed to parse cert");
        let sans = x509.subject_alt_names().expect("missing SAN");

        assert!(sans
            .iter()
            .any(|name| name.dnsname() == Some("example.com")));
    }

    #[test]
    fn ca_leaf_cert_contains_ip_san_for_ip_authority() {
        let material = RootCaMaterial::generate().expect("failed to generate CA");
        let ssl = Ssl::from_material(material).expect("failed to create ssl");
        let authority = Authority::from_static("127.0.0.1:443");

        let cert = ssl.gen_cert(&authority).expect("failed to generate cert");
        let x509 = X509::from_der(cert.0.as_slice()).expect("failed to parse cert");
        let sans = x509.subject_alt_names().expect("missing SAN");

        assert!(sans
            .iter()
            .any(|name| name.ipaddress() == Some([127, 0, 0, 1].as_slice())));
    }
}
