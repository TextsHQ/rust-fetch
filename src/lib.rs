use neon::prelude::*;

mod builder;
mod client;

use builder::Builder;
use client::Client;

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("clientRequest", Client::js_request)?;

    cx.export_function("builderNew", Builder::js_new)?;
    cx.export_function("builderConnectTimeout", Builder::js_connect_timeout)?;
    cx.export_function("builderRequestTimeout", Builder::js_request_timeout)?;
    cx.export_function("builderRedirectLimit", Builder::js_redirect_limit)?;
    cx.export_function("builderHttpsOnly", Builder::js_https_only)?;
    cx.export_function(
        "builderHttps2AdaptiveWindow",
        Builder::js_http2_adaptive_window,
    )?;
    cx.export_function("builderBuild", Builder::js_build)?;

    Ok(())
}
