use std::cell::RefCell;

use neon::prelude::*;

use tokio::runtime::Runtime;

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

    pub fn js_user_agent(mut cx: FunctionContext) -> JsResult<BoxedBuilder> {
        let user_agent = cx.argument::<JsString>(0)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let cb = rm.0.take().unwrap();

        Ok(JsBox::new(
            &mut cx,
            Self::containerize(cb.user_agent(user_agent)),
        ))
    }

    pub fn js_build(mut cx: FunctionContext) -> JsResult<JsBox<Client>> {
        let boxed = cx.this().downcast_or_throw::<BoxedBuilder, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let cb = rm.0.take().unwrap();

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
