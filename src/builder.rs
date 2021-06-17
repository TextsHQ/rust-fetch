use std::cell::RefCell;

use neon::prelude::*;

use tokio::runtime::Runtime;

use reqwest::redirect::Policy;
use reqwest::ClientBuilder;

use crate::client::Client;

pub struct Builder(Option<ClientBuilder>);

pub type BoxedBuilder = JsBox<RefCell<Builder>>;

impl Finalize for Builder {}

impl Builder {
    pub fn new() -> Self {
        Self(Some(ClientBuilder::new()))
    }

    pub fn containerize(cb: ClientBuilder) -> RefCell<Builder> {
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

        let cb = rm.0.take().unwrap();

        Ok(JsBox::new(
            &mut cx,
            Self::containerize(
                cb.connect_timeout(std::time::Duration::from_secs(duration_seconds as u64)),
            ),
        ))
    }

    pub fn js_request_timeout(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let duration_seconds = cx.argument::<JsNumber>(0)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let cb = rm.0.take().unwrap();

        Ok(JsBox::new(
            &mut cx,
            Self::containerize(cb.timeout(std::time::Duration::from_secs(duration_seconds as u64))),
        ))
    }

    pub fn js_https_only(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let only = cx.argument::<JsBoolean>(0)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let cb = rm.0.take().unwrap();

        Ok(JsBox::new(&mut cx, Self::containerize(cb.https_only(only))))
    }

    pub fn js_redirect_limit(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let limit = cx.argument::<JsNumber>(0)?.value(&mut cx) as usize;

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let cb = rm.0.take().unwrap();

        let policy = match limit {
            0 => Policy::none(),
            _ => Policy::limited(limit)
        };

        Ok(JsBox::new(&mut cx, Self::containerize(cb.redirect(policy))))
    }

    pub fn js_http2_adaptive_window(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let enabled = cx.argument::<JsBoolean>(0)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let cb = rm.0.take().unwrap();

        Ok(JsBox::new(
            &mut cx,
            Self::containerize(cb.http2_adaptive_window(enabled)),
        ))
    }

    pub fn js_build(mut cx: FunctionContext) -> JsResult<JsBox<Client>> {
        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let mut cb = rm.0.take().unwrap();

        cb = cb.tcp_keepalive(std::time::Duration::from_secs(60));

        let client = cb.build().unwrap();

        Ok(JsBox::new(
            &mut cx,
            Client {
                runtime: Runtime::new().unwrap(),
                client,
            },
        ))
    }
}
