use neon::prelude::*;

mod builder;
mod client;
mod jar;

use builder::Builder;
use client::Client;
use jar::Jar;

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("jarNew", Jar::js_new)?;
    cx.export_function("jarAddCookieStr", Jar::js_add_cookie_str)?;

    cx.export_function("clientRequest", Client::js_request)?;

    cx.export_function("builderNew", Builder::js_new)?;
    cx.export_function("builderJar", Builder::js_jar)?;
    cx.export_function("builderUserAgent", Builder::js_user_agent)?;
    cx.export_function("builderBuild", Builder::js_build)?;

    Ok(())
}
