use std::collections::HashMap;
use std::convert::TryInto;
use std::str::FromStr;
use std::sync::Arc;
use std::error::Error as StdError;
use std::time::{Duration, Instant};

use bytes::Bytes;

use futures_retry::{ErrorHandler, FutureRetry, RetryPolicy};

use log::{debug, trace};
use neon::prelude::*;
use neon::types::buffer::TypedArray;

use tokio::runtime::Runtime;

use reqwest::header::HeaderMap;
use reqwest::{Body, Client as ReqwestClient, Error, Method, Response};

use crate::time_jar::{TimeJar, NewCookies};

pub const RETRY_DURATION: Duration = Duration::from_millis(200);

pub struct Client {
    pub(crate) runtime: Runtime,

    pub(crate) client: ReqwestClient,

    pub(crate) time_jar: Arc<TimeJar>,
}

#[derive(Debug)]
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

pub enum HeaderEntry {
    Single(String),
    Multiple(Vec<String>),
}

pub struct CallbackPayload {
    status: f64,

    http_version: String,

    headers: HashMap<String, HeaderEntry>,

    content_length: Option<f64>,

    data: DataType,

    new_cookies: Vec<NewCookies>,
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
            let n: Handle<JsString> = names.get(cx, i)?;

            let v = obj.get_value(cx, n)?;

            match v {
                _ if v.is_a::<JsString, _>(cx) => {
                    let r = v.downcast_or_throw::<JsString, _>(cx)?.value(cx);

                    map.insert(n.value(cx), r);
                }

                _ if v.is_a::<JsNumber, _>(cx) => {
                    let i = v.downcast_or_throw::<JsNumber, _>(cx)?.value(cx);

                    map.insert(n.value(cx), (i as u64).to_string());
                }

                _ if v.is_a::<JsObject, _>(cx) => {
                    let key = n.value(cx);
                    cx.throw_error(format!("Object cannot be passed as a value, key: {}", key))?;
                }

                _ => {}
            };
        }

        Ok(map)
    }

    #[inline]
    pub fn object_keys(
        cx: &mut FunctionContext,
        obj: &Handle<JsObject>,
    ) -> NeonResult<HashMap<String, ()>> {
        let mut map = HashMap::new();

        let names = obj.get_own_property_names(cx)?;

        for i in 0..names.len(cx) {
            let n: Handle<JsString> = names.get(cx, i)?;

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
            let v: Vec<u8> = Vec::from(body.as_slice(cx));

            Ok(Some(Body::from(v)))
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
        new_cookies: Vec<NewCookies>,
    ) -> Result<CallbackPayload, reqwest::Error> {
        match res {
            Ok(res) => {
                let status = res.status().as_u16() as f64;
                let http_version = format!("{:?}", res.version());

                let mut headers: HashMap<String, HeaderEntry> = HashMap::new();

                for key in res.headers().keys() {
                    let value_entries = res
                        .headers()
                        .get_all(key)
                        .iter()
                        .filter(|v| v.to_str().is_ok()) // Maybe FIXME: This may be seen as a quirk if non-ascii headers are omitted
                        .map(|v| v.to_str().unwrap().to_string())
                        .collect::<Vec<String>>();

                    match value_entries.len() {
                        1 => headers.insert(
                            key.to_string(),
                            HeaderEntry::Single(value_entries[0].clone()),
                        ),
                        _ => headers.insert(key.to_string(), HeaderEntry::Multiple(value_entries)),
                    };
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
                    new_cookies,
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
                match v {
                    HeaderEntry::Single(s) => {
                        let s = cx.string(s);

                        h.set(cx, k.as_ref(), s)?;
                    }
                    HeaderEntry::Multiple(entries) => {
                        let val = JsArray::new(cx, entries.len() as u32);

                        for (i, entry) in entries.iter().enumerate() {
                            let z = cx.string(entry);

                            val.set(cx, i as u32, z)?;
                        }

                        h.set(cx, k.as_ref(), val)?;
                    }
                };
            }

            h
        };

        let new_cookies = {
            let h = JsObject::new(cx);

            for (k, v) in payload.new_cookies {
                let val = JsArray::new(cx, v.len() as u32);

                for (i, entry) in v.iter().enumerate() {
                    let z = cx.string(entry.to_string());

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

                let mut buf= JsBuffer::new(cx, val.len())?;
                buf.as_mut_slice(cx).copy_from_slice(&val);

                obj.set(cx, "body", buf)?;
            }

            _ => {}
        }

        let status = cx.number(payload.status);
        let http_version = cx.string(payload.http_version);

        obj.set(cx, "statusCode", status)?;
        obj.set(cx, "httpVersion", http_version)?;
        obj.set(cx, "headers", headers)?;
        obj.set(cx, "newCookies", new_cookies)?;

        Ok(obj)
    }

    pub fn js_request(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let url = cx.argument::<JsString>(0)?.value(&mut cx);
        let args = cx.argument::<JsObject>(1)?;
        let callback = cx.argument::<JsFunction>(2)?.root(&mut cx);

        let this = cx.this().downcast_or_throw::<JsBox<Self>, _>(&mut cx)?;

        let keys = Self::object_keys(&mut cx, &args)?;

        let method = Method::from_str(&args.get::<JsString, _, _>(&mut cx, "method")?.value(&mut cx)).unwrap();

        let attempts = args.get::<JsNumber, _, _>(&mut cx, "attempts")?.value(&mut cx) as usize;

        debug!(
            "Received {} request to {} with {} attempts",
            &method, &url, &attempts
        );

        let mut builder = this.client.request(method.clone(), url);

        if keys.contains_key("headers") {
            let headers: Handle<JsObject> = args.get(&mut cx, "headers")?;
            let headers = Self::map_jsobject(&mut cx, &headers)?;
            let headers: HeaderMap = match (&headers).try_into() {
                Ok(v) => v,
                Err(e) => cx.throw_error(format!("Invalid headers: {}", e))?,
            };

            debug!("Request headers: {:?}", &headers);

            builder = builder.headers(headers);
        }

        if keys.contains_key("body") {
            let body = args.get(&mut cx, "body")?;

            if let Some(body) = Self::map_body(&mut cx, body)? {
                trace!("Request body: {:?}", &body);
                builder = builder.body(body);
            }
        }

        if keys.contains_key("searchParams") {
            let search_params: Handle<JsObject> = args.get(&mut cx, "searchParams")?;
            let search_params = Self::map_jsobject(&mut cx, &search_params)?;

            debug!("Request queries: {:?}", &search_params);

            builder = builder.query(&search_params);
        }

        if keys.contains_key("form") {
            let form: Handle<JsObject> = args.get(&mut cx, "form")?;
            let form = Self::map_jsobject(&mut cx, &form)?;

            debug!("Request form: {:?}", form);

            builder = builder.form(&form);
        }

        let response_type = ResponseType::from_str(&args.get::<JsString, _, _>(&mut cx, "responseType")?.value(&mut cx)).unwrap();

        debug!("Request response type: {:?}", &response_type);

        let queue = cx.channel();

        let time_jar = this.time_jar.clone();

        this.runtime.spawn(async move {
            let request_time = Instant::now();

            let res = FutureRetry::new(
                || builder.try_clone().unwrap().send(),
                Attempter::new(method, attempts),
            )
            .await
            .map_err(|(e, attempts)| {
                debug!("Request error after {} attempts: {}", attempts, e);
                e
            })
            .map(|(r, attempts)| {
                debug!("Request successful after {} attempts", attempts);
                r
            });

            let new_cookies = time_jar.cookies_since(request_time);

            let res = Self::map_response(res, response_type, new_cookies).await;

            queue.send(|mut cx| {
                let cb = callback.into_inner(&mut cx);
                let this = cx.undefined();

                match res {
                    Ok(v) => {
                        let ret = Self::build_ret(&mut cx, v)?;

                        let args: Vec<Handle<JsValue>> = vec![cx.null().upcast(), ret.upcast()];

                        debug!("Called back with successful response");

                        cb.call(&mut cx, this, args)?;
                    }
                    Err(e) => {
                        let args: Vec<Handle<JsValue>> = vec![cx.error(e.to_string())?.upcast()];

                        debug!("Called back with error");

                        cb.call(&mut cx, this, args)?;
                    }
                };

                Ok(())
            });
        });

        Ok(cx.undefined())
    }
}

pub struct Attempter {
    method: Method,

    attempts: usize,
    max_attempts: usize,
}

impl Attempter {
    pub fn new(method: Method, attempts: usize) -> Self {
        Self {
            method,
            attempts: 0,
            max_attempts: attempts,
        }
    }
}

impl ErrorHandler<Error> for Attempter {
    type OutError = Error;

    fn handle(&mut self, _attempt: usize, e: Error) -> RetryPolicy<Self::OutError> {
        if self.attempts >= self.max_attempts {
            debug!(
                "Reached max attempts of {}, forwarding error",
                &self.max_attempts
            );
            return RetryPolicy::ForwardError(e);
        }

        // Check if the error is io::ErrorKind::BrokenPipe
        let mut source = e.source();
        let mut is_broken_pipe = false;

        while let Some(err_source) = source {
            if let Some(io_error) = err_source.downcast_ref::<std::io::Error>() {
                if io_error.kind() == std::io::ErrorKind::BrokenPipe {
                    is_broken_pipe = true;
                    break;
                }
            }

            source = err_source.source();
        }

        // https://datatracker.ietf.org/doc/html/rfc7231#section-4.2.1
        if !self.method.is_idempotent() && !e.is_connect() && !is_broken_pipe {
            debug!("Request method is non-idempotent, forwarding error");
            return RetryPolicy::ForwardError(e);
        }

        self.attempts += 1;

        let retry_duration = RETRY_DURATION * self.attempts as u32;

        match e {
            _ if e.is_connect() => {
                debug!("Request connection error, retrying");
                RetryPolicy::WaitRetry(retry_duration)
            }
            _ if e.is_timeout() => {
                debug!("Request timeout error, retrying");
                RetryPolicy::WaitRetry(retry_duration)
            }
            _ if e.is_status() => {
                let status = e.status().unwrap();

                // https://github.com/sindresorhus/got/#retry
                match status.as_u16() {
                    408 | 413 | 429 | 500 | 502 | 503 | 504 | 521 | 522 | 524 => {
                        debug!("Request status error: {}, retrying", &status);
                        RetryPolicy::WaitRetry(retry_duration)
                    }
                    _ => {
                        debug!("Request status error: {}, forwarding error", &status);
                        RetryPolicy::ForwardError(e)
                    }
                }
            }

            _ => {
                debug!("Request error: {}, retrying", &e);
                RetryPolicy::WaitRetry(retry_duration)
            }
        }
    }
}
