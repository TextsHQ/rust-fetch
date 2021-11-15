use std::time::Instant;
use std::ops::Bound::{Excluded, Unbounded};
use std::sync::RwLock;
use std::collections::BTreeMap;

use reqwest::Url;
use reqwest::header::HeaderValue;
use reqwest::cookie::CookieStore;

pub type NewCookies = (String, Vec<String>);

/// Time based jar.
///
/// The motivation behind is that we cannot set a dedicate jar for each request.
/// And recreating the client for each request seems wasteful.
///
/// Therefore we need to track cookies by time to fetch cookies since request time.
///
/// This is rather crude, but it works, and be later improved when better support lands in reqwest.
/// Namely: https://github.com/seanmonstar/reqwest/issues/353
pub struct TimeJar(RwLock<BTreeMap<Instant, NewCookies>>);

impl TimeJar {
    pub fn cookies_since(&self, time: Instant) -> Vec<NewCookies> {
        let mut cookies = Vec::new();
        let jar = self.0.read().unwrap();

        for (_t, v) in jar.range((Excluded(time), Unbounded)) {
            cookies.push(v.to_owned())
        }

        cookies
    }

}

impl Default for TimeJar {
    fn default() -> Self {
        TimeJar(RwLock::new(BTreeMap::new()))
    }
}

impl CookieStore for TimeJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let mut jar = self.0.write().unwrap();

        let cookies = cookie_headers.map(|h| h.to_str().unwrap().to_owned()).collect();

        jar.insert(Instant::now(), (url.origin().ascii_serialization(), cookies));
    }

    // Time jar is not designed to serve cookies for requests, that is the job of the JS caller.
    fn cookies(&self, _url: &Url) -> Option<HeaderValue> {
        None
    }
}
