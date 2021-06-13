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
            "BINARY" => Ok(ResponseType::Binary),

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
                    ResponseType::Binary => DataType::Binary(res.bytes().await.ok())
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
            },

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

        let obj = args
            .get(&mut cx, "form")?
            .downcast_or_throw::<JsObject, _>(&mut cx)?;
        let form = Self::map_jsobject(&mut cx, obj)?;
        builder = builder.form(&form);

        let response_type = ResponseType::from_str(
            &args
                .get(&mut cx, "responseType")?
                .downcast_or_throw::<JsString, _>(&mut cx)?
                .value(&mut cx)
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
