use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::TryInto;
use std::str::FromStr;

use bytes::Bytes;

use neon::prelude::*;

use tokio::runtime::Runtime;

use reqwest::header::HeaderMap;
use reqwest::{Body, Client as ReqwestClient, Method, Response};

pub struct Client {
    pub(crate) runtime: Runtime,

    pub(crate) client: ReqwestClient,
}

pub enum ResponseType {
    Text,
    Binary,
}

impl FromStr for ResponseType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "binary" => Ok(ResponseType::Binary),

            // Defaults to text, even for invalid cases.
            _ => Ok(ResponseType::Text),
        }
    }
}

pub enum DataType {
    Text(Option<String>),
    Binary(Option<Bytes>),
}

pub struct CallbackPayload {
    status: f64,

    http_version: String,

    headers: HashMap<String, Vec<String>>,

    content_length: Option<f64>,

    data: DataType,
}

impl Finalize for Client {}

impl Client {
    #[inline]
    pub fn map_jsobject(
        cx: &mut FunctionContext,
        obj: &Handle<JsObject>,
    ) -> NeonResult<HashMap<String, String>> {
        let mut map = HashMap::new();

        let names = obj.get_own_property_names(cx)?;

        for i in 0..names.len(cx) {
            let n = names.get(cx, i)?.downcast::<JsString, _>(cx).or_throw(cx)?;

            let v = obj.get(cx, n)?;

            let v = if v.is_a::<JsString, _>(cx) {
                v.downcast_or_throw::<JsString, _>(cx)?.value(cx)
            } else {
                let i = v.downcast_or_throw::<JsNumber, _>(cx)?.value(cx);

                (i as u32).to_string()
            };

            map.insert(n.value(cx), v);
        }

        Ok(map)
    }

    #[inline]
    pub fn object_keys(cx: &mut FunctionContext, obj: &Handle<JsObject>) -> NeonResult<HashMap<String, ()>> {
        let mut map = HashMap::new();

        let names = obj.get_own_property_names(cx)?;

        for i in 0..names.len(cx) {
            let n = names.get(cx, i)?.downcast::<JsString, _>(cx).or_throw(cx)?;

            map.insert(n.value(cx), ());
        }

        Ok(map)
    }

    /// Maps & check a JS type to `Body`.
    /// A body could be either a string or a JsBuffer if provided.
    #[inline]
    pub fn map_body(cx: &mut FunctionContext, body: Handle<JsValue>) -> NeonResult<Option<Body>> {
        if body.is_a::<JsString, _>(cx) {
            let body = body.downcast_or_throw::<JsString, _>(cx)?.value(cx);

            Ok(Some(Body::from(body)))
        } else if body.is_a::<JsBuffer, _>(cx) {
            let body = body.downcast_or_throw::<JsBuffer, _>(cx)?;

            cx.borrow(&body, |data| {
                let v: Vec<u8> = Vec::from(data.as_slice());

                Ok(Some(Body::from(v)))
            })
        } else {
            Ok(None)
        }
    }

    /// Maps a response to inner data payload, essentially a copy and transform.
    /// Due to non-Send nature of FunctionContext, and non async of queue send fn prototype.
    #[inline]
    pub async fn map_response(
        res: Result<Response, reqwest::Error>,
        response_type: ResponseType,
    ) -> Result<CallbackPayload, reqwest::Error> {
        match res {
            Ok(res) => {
                let status = res.status().as_u16() as f64;
                let http_version = format!("{:?}", res.version());

                let mut headers: HashMap<String, Vec<String>> = HashMap::new();

                for (name, val) in res.headers() {
                    // Maybe FIXME: This may be seen as a quirk if non-ascii headers are omitted
                    if let Ok(val_str) = val.to_str() {
                        let name = name.to_string();

                        match headers.entry(name) {
                            Entry::Occupied(o) => o.into_mut().push(val_str.to_string()),
                            Entry::Vacant(v) => {
                                v.insert(vec![val_str.to_string()]);
                            }
                        };
                    }
                }

                let content_length = res.content_length().map(|i| i as f64);

                let data = match response_type {
                    ResponseType::Text => DataType::Text(res.text().await.ok()),
                    ResponseType::Binary => DataType::Binary(res.bytes().await.ok()),
                };

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

    #[inline]
    pub fn build_ret<'c>(
        cx: &mut TaskContext<'c>,
        payload: CallbackPayload,
    ) -> NeonResult<Handle<'c, JsObject>> {
        let obj = JsObject::new(cx);

        let headers = {
            let h = JsObject::new(cx);

            for (k, v) in payload.headers {
                let val = JsArray::new(cx, v.len() as u32);

                for (i, entry) in v.iter().enumerate() {
                    let z = cx.string(entry);

                    val.set(cx, i as u32, z)?;
                }

                h.set(cx, k.as_ref(), val)?;
            }

            h
        };

        if let Some(content_length) = payload.content_length {
            let val = cx.number(content_length);

            obj.set(cx, "contentLength", val)?;
        }

        match payload.data {
            DataType::Text(v) if v.is_some() => {
                let val = cx.string(v.unwrap());

                obj.set(cx, "body", val)?;
            }

            DataType::Binary(v) if v.is_some() => {
                let val = v.unwrap();

                let mut buf = JsBuffer::new(cx, val.len() as u32)?;

                cx.borrow_mut(&mut buf, |data| {
                    data.as_mut_slice::<u8>().copy_from_slice(&val);
                });

                obj.set(cx, "body", buf)?;
            }

            _ => {}
        }

        let status = cx.number(payload.status);
        let http_version = cx.string(payload.http_version);

        obj.set(cx, "statusCode", status)?;
        obj.set(cx, "httpVersion", http_version)?;
        obj.set(cx, "headers", headers)?;

        Ok(obj)
    }

    pub fn js_request(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let url = cx.argument::<JsString>(0)?.value(&mut cx);
        let args = cx.argument::<JsObject>(1)?;
        let callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let this = cx.this().downcast_or_throw::<JsBox<Self>, _>(&mut cx)?;

        let keys = Self::object_keys(&mut cx, &args)?;

        let method = Method::from_str(
            &args
                .get(&mut cx, "method")?
                .downcast_or_throw::<JsString, _>(&mut cx)?
                .value(&mut cx),
        )
        .unwrap();

        let mut builder = this.client.request(method, url);

        if keys.contains_key("headers") {
            let headers = args.get(&mut cx, "headers")?.downcast_or_throw::<JsObject, _>(&mut cx)?;
            let headers = Self::map_jsobject(&mut cx, &headers)?;
            let headers: HeaderMap = match (&headers).try_into() {
                Ok(v) => v,
                Err(e) => cx.throw_error(format!("Invalid headers: {}", e))?,
            };
            builder = builder.headers(headers);
        }

        if keys.contains_key("body") {
            let body = args.get(&mut cx, "body")?;

            if let Some(body) = Self::map_body(&mut cx, body)? {
                builder = builder.body(body);
            }
        }

        if keys.contains_key("searchParams") {
            let search_params = args.get(&mut cx, "searchParams")?.downcast_or_throw::<JsObject, _>(&mut cx)?;
            let search_params = Self::map_jsobject(&mut cx, &search_params)?;
            builder = builder.query(&search_params);
        }

        if keys.contains_key("form") {
            let form = args.get(&mut cx, "form")?.downcast_or_throw::<JsObject, _>(&mut cx)?;
            let form = Self::map_jsobject(&mut cx, &form)?;
            builder = builder.form(&form);
        }

        let response_type = ResponseType::from_str(
            &args
                .get(&mut cx, "responseType")?
                .downcast_or_throw::<JsString, _>(&mut cx)?
                .value(&mut cx),
        )
        .unwrap();

        let queue = cx.channel();

        this.runtime.spawn(async move {
            let res = builder.send().await;

            let res = Self::map_response(res, response_type).await;

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
