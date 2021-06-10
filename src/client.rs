use std::collections::HashMap;
use std::convert::TryInto;
use std::str::FromStr;

use neon::prelude::*;

use tokio::runtime::Runtime;

use reqwest::header::HeaderMap;
use reqwest::{Body, Client as ReqwestClient, Method, Response};

pub struct Client {
    pub(crate) runtime: Runtime,

    pub(crate) client: ReqwestClient,
}

pub struct CallbackPayload {
    status: f64,
    http_version: String,
    headers: Vec<(String, String)>,
    content_length: Option<f64>,
    data: Option<String>,
}

impl Finalize for Client {}

impl Client {
    #[inline]
    pub fn map_jsobject(
        cx: &mut FunctionContext,
        obj: Handle<JsObject>,
    ) -> NeonResult<HashMap<String, String>> {
        let mut map = HashMap::new();

        let names = obj.get_own_property_names(cx)?;

        for i in 0..names.len(cx) {
            let n = names.get(cx, i)?.downcast::<JsString, _>(cx).or_throw(cx)?;

            let v = obj
                .get(cx, n)?
                .downcast::<JsString, _>(cx)
                .or_throw(cx)?
                .value(cx);

            map.insert(n.value(cx), v);
        }

        Ok(map)
    }

    /// Maps & check a JS type to `Body`.
    /// A body could be either a string or a JsBuffer if provided.
    #[inline]
    pub fn map_body(cx: &mut FunctionContext, body: Handle<JsValue>) -> NeonResult<Body> {
        if body.is_a::<JsString, _>(cx) {
            let body = body.downcast::<JsString, _>(cx).or_throw(cx)?.value(cx);

            Ok(Body::from(body))
        } else {
            let body = body.downcast::<JsBuffer, _>(cx).or_throw(cx)?;

            cx.borrow(&body, |data| {
                let v: Vec<u8> = Vec::from(data.as_slice());

                Ok(Body::from(v))
            })
        }
    }

    /// Maps a response to inner data payload, essentially a copy and transform.
    /// Due to non-Send nature of FunctionContext, and non async of queue send fn prototype.
    #[inline]
    pub async fn map_response(
        res: Result<Response, reqwest::Error>,
    ) -> Result<CallbackPayload, reqwest::Error> {
        match res {
            Ok(res) => {
                let status = res.status().as_u16() as f64;
                let http_version = format!("{:?}", res.version());

                let headers = res
                    .headers()
                    .iter()
                    .filter(|(_name, val)| val.to_str().is_ok()) // FIXME: This may be seen as a quirk if non-ascii headers are omitted
                    .map(|(name, val)| (name.as_str().to_owned(), val.to_str().unwrap().to_owned()))
                    .collect::<Vec<(String, String)>>();

                let content_length = res.content_length().map(|i| i as f64);

                let data = res.text().await.ok();

                Ok(CallbackPayload {
                    status,
                    http_version,
                    headers,
                    content_length,
                    data,
                })
            }

            Err(e) => Err(e),
        }
    }

    pub fn build_ret<'c>(
        cx: &mut TaskContext<'c>,
        payload: CallbackPayload,
    ) -> NeonResult<Handle<'c, JsObject>> {
        let obj = JsObject::new(cx);

        let headers = {
            let h = JsObject::new(cx);

            for (k, v) in payload.headers {
                let val = cx.string(v);

                h.set(cx, k.as_ref(), val)?;
            }

            h
        };

        if let Some(content_length) = payload.content_length {
            let val = cx.number(content_length);

            obj.set(cx, "contentLength", val)?;
        }

        if let Some(data) = payload.data {
            let val = cx.string(data);

            obj.set(cx, "data", val)?;
        }

        let status = cx.number(payload.status);
        let http_version = cx.string(payload.http_version);

        obj.set(cx, "status", status)?;
        obj.set(cx, "httpVersion", http_version)?;
        obj.set(cx, "headers", headers)?;

        Ok(obj)
    }

    pub fn js_request(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let url = cx.argument::<JsString>(0)?.value(&mut cx);
        let args = cx.argument::<JsObject>(1)?;
        let callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let this = cx.this().downcast_or_throw::<JsBox<Self>, _>(&mut cx)?;

        let method = Method::from_str(
            &args
                .get(&mut cx, "method")?
                .downcast::<JsString, _>(&mut cx)
                .or_throw(&mut cx)?
                .value(&mut cx),
        )
        .unwrap();

        let mut builder = this.client.request(method, url);

        let headers = args
            .get(&mut cx, "headers")?
            .downcast_or_throw::<JsObject, _>(&mut cx)?;
        let headers = Self::map_jsobject(&mut cx, headers)?;
        let headers: HeaderMap = match (&headers).try_into() {
            Ok(v) => v,
            Err(e) => cx.throw_error(format!("Invalid headers: {}", e))?,
        };
        builder = builder.headers(headers);

        let body = args.get(&mut cx, "body")?;
        let body = Self::map_body(&mut cx, body)?;
        builder = builder.body(body);

        let obj = args
            .get(&mut cx, "query")?
            .downcast_or_throw::<JsObject, _>(&mut cx)?;
        let query = Self::map_jsobject(&mut cx, obj)?;
        builder = builder.query(&query);

        let queue = cx.channel();

        this.runtime.spawn(async move {
            let res = builder.send().await;

            let res = Self::map_response(res).await;

            queue.send(|mut cx| {
                let ret = match res {
                    Ok(v) => v,
                    Err(e) => cx.throw_error(format!("Request error: {}", e))?,
                };

                let ret = Self::build_ret(&mut cx, ret)?;

                let cb = callback.into_inner(&mut cx);

                let this = cx.undefined();

                let args: Vec<Handle<JsValue>> = vec![cx.null().upcast(), ret.upcast()];

                cb.call(&mut cx, this, args)?;

                Ok(())
            });
        });

        Ok(cx.undefined())
    }
}
