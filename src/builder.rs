use std::cell::RefCell;

use env_logger::Builder as LoggerBuilder;
use log::LevelFilter;

use neon::prelude::*;

use tokio::runtime::Runtime;

use rustls::ClientConfig;

use reqwest::redirect::Policy;
use reqwest::{ClientBuilder, Proxy};

use crate::client::Client;
use crate::time_jar::TimeJar;

pub struct Builder(Option<BuilderInner>);

pub struct BuilderInner {
    client: ClientBuilder,

    log_level: LevelFilter,
}

impl BuilderInner {
    pub fn new() -> BuilderInner {
        Self {
            client: ClientBuilder::new(),
            log_level: LevelFilter::Info,
        }
    }
}

pub type BoxedBuilder = JsBox<RefCell<Builder>>;

impl Finalize for Builder {}

impl Builder {
    pub fn new() -> Self {
        Self(Some(BuilderInner::new()))
    }

    pub fn containerize(cb: BuilderInner) -> RefCell<Builder> {
        RefCell::new(Self(Some(cb)))
    }
}

/// Neon bindings for `Builder`.
impl Builder {
    pub fn js_new(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        Ok(JsBox::new(&mut cx, RefCell::new(Self::new())))
    }

    pub fn js_connect_timeout(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let duration_seconds = cx.argument::<JsNumber>(0)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let mut cb = rm.0.take().unwrap();
        cb.client = cb
            .client
            .connect_timeout(std::time::Duration::from_secs(duration_seconds as u64));

        Ok(JsBox::new(&mut cx, Self::containerize(cb)))
    }

    pub fn js_request_timeout(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let duration_seconds = cx.argument::<JsNumber>(0)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let mut cb = rm.0.take().unwrap();
        cb.client = cb
            .client
            .timeout(std::time::Duration::from_secs(duration_seconds as u64));

        Ok(JsBox::new(&mut cx, Self::containerize(cb)))
    }

    pub fn js_https_only(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let only = cx.argument::<JsBoolean>(0)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let mut cb = rm.0.take().unwrap();
        cb.client = cb.client.https_only(only);

        Ok(JsBox::new(&mut cx, Self::containerize(cb)))
    }

    pub fn js_strip_sensitive_headers(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let strip = cx.argument::<JsBoolean>(0)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let mut cb = rm.0.take().unwrap();
        cb.client = cb.client.strip_sensitive_headers(strip);

        Ok(JsBox::new(&mut cx, Self::containerize(cb)))
    }

    pub fn js_redirect_limit(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let limit = cx.argument::<JsNumber>(0)?.value(&mut cx) as usize;

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let mut cb = rm.0.take().unwrap();

        let policy = match limit {
            0 => Policy::none(),
            _ => Policy::limited(limit),
        };

        cb.client = cb.client.redirect(policy);

        Ok(JsBox::new(&mut cx, Self::containerize(cb)))
    }

    pub fn js_http2_adaptive_window(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let enabled = cx.argument::<JsBoolean>(0)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let mut cb = rm.0.take().unwrap();
        cb.client = cb.client.http2_adaptive_window(enabled);

        Ok(JsBox::new(&mut cx, Self::containerize(cb)))
    }

    pub fn js_proxy(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let proxy = cx.argument::<JsString>(0)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let mut cb = rm.0.take().unwrap();
        cb.client = cb.client.proxy(Proxy::all(proxy).unwrap());

        Ok(JsBox::new(&mut cx, Self::containerize(cb)))
    }

    pub fn js_log_level(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let level = cx.argument::<JsNumber>(0)?.value(&mut cx) as u64;

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let mut cb = rm.0.take().unwrap();

        cb.log_level = match level {
            0 => LevelFilter::Off,
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            5 => {
                cb.client = cb.client.connection_verbose(true);

                LevelFilter::Trace
            }

            _ => LevelFilter::Info,
        };

        // Since texts only have one global rust-fetch instance,
        // a global logger instance should be fine, also needed to capture connection verbose.
        LoggerBuilder::new().filter_level(cb.log_level).try_init().ok();

        Ok(JsBox::new(&mut cx, Self::containerize(cb)))
    }

    pub fn js_build(mut cx: FunctionContext) -> JsResult<JsBox<Client>> {
        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let mut cb = rm.0.take().unwrap();

        cb.client = cb.client.http2_initial_stream_window_size(1024 * 256 * 24);

        let time_jar = std::sync::Arc::new(TimeJar::default());

        cb.client = cb.client.cookie_provider(time_jar.clone());

        cb.client = cb.client.use_preconfigured_tls({
            let mut config = ClientConfig::builder()
                .with_cipher_suites(&[
                    // GREASE
                    rustls::cipher_suite::TLS13_AES_128_GCM_SHA256,
                    rustls::cipher_suite::TLS13_AES_256_GCM_SHA384,
                    rustls::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
                    rustls::cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
                    rustls::cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
                    rustls::cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
                    rustls::cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
                    rustls::cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
                    rustls::cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
                    // unsupported
                    // rustls::cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA,
                    // rustls::cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA,
                    // rustls::cipher_suite::TLS_RSA_WITH_AES_128_GCM_SHA256,
                    // rustls::cipher_suite::TLS_RSA_WITH_AES_256_GCM_SHA384,
                    // rustls::cipher_suite::TLS_RSA_WITH_AES_128_CBC_SHA,
                    // rustls::cipher_suite::TLS_RSA_WITH_AES_256_CBC_SHA,
                ])
                .with_kx_groups(&[
                    // GREASE
                    &rustls::kx_group::X25519,
                    &rustls::kx_group::SECP256R1,
                    &rustls::kx_group::SECP384R1,
                ])
                .with_protocol_versions(&[
                    // GREASE
                    &rustls::version::TLS12,
                    &rustls::version::TLS13,
                ])
                .unwrap()
                .with_root_certificates({
                    let mut root_cert_store = rustls::RootCertStore::empty();

                    let mut valid_count = 0;
                    let mut invalid_count = 0;

                    for cert in rustls_native_certs::load_native_certs().unwrap() {
                        let cert = rustls::Certificate(cert.0);
                        // Continue on parsing errors, as native stores often include ancient or syntactically
                        // invalid certificates, like root certificates without any X509 extensions.
                        // Inspiration: https://github.com/rustls/rustls/blob/633bf4ba9d9521a95f68766d04c22e2b01e68318/rustls/src/anchors.rs#L105-L112
                        match root_cert_store.add(&cert) {
                            Ok(_) => valid_count += 1,
                            Err(err) => {
                                invalid_count += 1;
                                log::warn!(
                                    "rustls failed to parse DER certificate {:?} {:?}",
                                    &err,
                                    &cert
                                );
                            }
                        }
                    }

                    log::info!(
                        "rustls_native_certs loaded {} valid certificates and {} invalid certificates",
                        valid_count,
                        invalid_count
                    );

                    root_cert_store
                })
                .with_no_client_auth();
            config.alpn_protocols = vec!["h2".into(), "http/1.1".into()];
            config
        });

        let client = cb.client.build().unwrap();

        Ok(JsBox::new(
            &mut cx,
            Client {
                runtime: Runtime::new().unwrap(),
                client,
                time_jar,
            },
        ))
    }
}
