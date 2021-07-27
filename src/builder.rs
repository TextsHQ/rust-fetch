use std::cell::RefCell;

use neon::prelude::*;

use tokio::runtime::Runtime;

use reqwest::redirect::Policy;
use reqwest::ClientBuilder;

use crate::client::Client;

pub struct Builder(Option<BuilderInner>);

pub struct BuilderInner {
    client: ClientBuilder,

    verbose: bool,
}

impl BuilderInner {
    pub fn new() -> BuilderInner {
        Self {
            client: ClientBuilder::new(),
            verbose: false,
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

    pub fn js_verbose(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let enabled = cx.argument::<JsBoolean>(0)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let mut cb = rm.0.take().unwrap();
        cb.verbose = enabled;

        Ok(JsBox::new(&mut cx, Self::containerize(cb)))
    }

    pub fn js_build(mut cx: FunctionContext) -> JsResult<JsBox<Client>> {
        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let cb = rm.0.take().unwrap();

        let client = cb.client.build().unwrap();

        Ok(JsBox::new(
            &mut cx,
            Client {
                runtime: Runtime::new().unwrap(),
                client,
                verbose: cb.verbose,
            },
        ))
    }
}
