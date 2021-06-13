use neon::prelude::*;

mod builder;
mod client;

use builder::Builder;
use client::Client;

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("clientRequest", Client::js_request)?;

    cx.export_function("builderNew", Builder::js_new)?;
    cx.export_function("builderUserAgent", Builder::js_user_agent)?;
    cx.export_function("builderBuild", Builder::js_build)?;

    Ok(())
}
