use std::cell::RefCell;

use neon::prelude::*;

use reqwest::cookie::Jar as ReqwestJar;
use reqwest::Url;

pub struct Jar(pub(crate) Option<ReqwestJar>);

pub type BoxedJar = JsBox<RefCell<Jar>>;

impl Finalize for Jar {}

impl Jar {
    pub fn new() -> Self {
        Self(Some(ReqwestJar::default()))
    }
}

impl Jar {
    pub fn js_new(mut cx: FunctionContext) -> JsResult<BoxedJar> {
        Ok(JsBox::new(&mut cx, RefCell::new(Self::new())))
    }

    pub fn js_add_cookie_str(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        // Cookie is a string of kv pair "foo=bar"
        let cookie = cx.argument::<JsString>(0)?.value(&mut cx);
        let url = cx.argument::<JsString>(1)?.value(&mut cx);

        let boxed = cx.this().downcast_or_throw::<BoxedJar, _>(&mut cx)?;

        let mut rm = boxed.borrow_mut();

        let jar = rm.0.take().unwrap();

        // TODO: Ret error instead of unwrap
        jar.add_cookie_str(&cookie, &Url::parse(&url).unwrap());

        Ok(cx.undefined())
    }
}
