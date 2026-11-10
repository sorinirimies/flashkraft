//! JSON and static routes.

use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, HeaderValue, StatusCode,
        content::{Css, Js, Json},
        header, query_params,
        request::headers,
        route,
    },
};

use crate::{
    data::{inspect_image, load},
    flash::{self, FlashStatus, Reject, StartRequest, TOKEN_HEADER},
    params::{BrowseParams, PageParams},
    style::CSS,
};

type Reply = (StatusCode, Json<serde_json::Value>);

fn reply(r: Result<(), Reject>) -> Reply {
    match r {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))),
        Err(e) => (
            StatusCode::from_u16(e.status).unwrap_or(StatusCode::BAD_REQUEST),
            Json(serde_json::json!({ "ok": false, "error": e.message })),
        ),
    }
}

/// Guards 1–3 on the incoming request.
fn guard(cx: &Cx) -> Result<(), Reject> {
    let h = headers(cx);
    let get = |n: &str| h.get(n).and_then(|v| v.to_str().ok());
    flash::check_caller(flash::config(), get("host"), get(TOKEN_HEADER))
}

/// Current flash job (read-only, no token needed).
#[route(GET "/api/flash")]
async fn flash_status() -> Result<Json<FlashStatus>> {
    Ok(Json(flash::manager().status()))
}

/// Start flashing. See [`crate::flash`] for the guards.
#[route(POST "/api/flash")]
async fn flash_start(cx: &Cx, Json(req): Json<StartRequest>) -> Result<Reply> {
    if let Err(e) = guard(cx) {
        return Ok(reply(Err(e)));
    }
    // Re-detect everything on the server; never trust the browser's list.
    let image_path = req.image.clone();
    let checked = tokio::task::spawn_blocking(move || {
        let found = flashkraft_core::commands::load_drives_sync();
        (found, inspect_image(&image_path))
    })
    .await;
    let Ok((found, image)) = checked else {
        return Ok(reply(Err(Reject {
            status: 500,
            message: "drive detection failed".into(),
        })));
    };
    let result = flash::validate(&req, &found, image)
        .and_then(|(image, targets)| flash::manager().start(&image, &targets));
    Ok(reply(result))
}

/// Stop the running flash.
#[route(POST "/api/flash/cancel")]
async fn flash_cancel(cx: &Cx) -> Result<Reply> {
    if let Err(e) = guard(cx) {
        return Ok(reply(Err(e)));
    }
    flash::manager().cancel();
    Ok(reply(Ok(())))
}

#[route(GET "/flash.js")]
async fn flash_js() -> Result<Js<&'static str>> {
    Ok(Js(flash::SCRIPT))
}

/// Drives (and fit for `?image=`) plus history, as JSON.
#[route(GET "/api/drives")]
async fn drives(cx: &Cx) -> Result<Json<crate::data::Snapshot>> {
    let params = query_params::<PageParams>(cx)?;
    Ok(Json(load(params.image_path()).await))
}

/// List a directory for the image picker (needs the `Host` + token guards).
#[route(GET "/api/browse")]
async fn browse(cx: &Cx) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let params = query_params::<BrowseParams>(cx)?;
    if let Err(e) = guard(cx) {
        return Ok(reply(Err(e)));
    }
    let path = params.path.clone().unwrap_or_default();
    let listing = tokio::task::spawn_blocking(move || crate::browse::list_dir(&path)).await;
    Ok(match listing {
        Ok(Ok(l)) => (StatusCode::OK, Json(serde_json::json!(l))),
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "browse failed" })),
        ),
    })
}

/// Flash history of the GUI and TUI.
#[route(GET "/api/history")]
async fn history() -> Result<Json<Vec<crate::data::HistoryRow>>> {
    Ok(Json(load(None).await.history))
}

/// Liveness probe.
#[route(GET "/api/health")]
async fn health() -> Result<Json<&'static str>> {
    Ok(Json("ok"))
}

/// Every colour theme with its palette (the same ones the TUI and GUI offer).
#[route(GET "/api/themes")]
async fn themes() -> Result<Json<serde_json::Value>> {
    Ok(Json(crate::theme::catalogue()))
}

/// The app icon (the master SVG), used by the header and as the favicon.
const ICON_SVG: &str = include_str!("../assets/flashkraft.svg");

#[route(GET "/icon.svg")]
async fn icon() -> Result<([(header::HeaderName, HeaderValue); 2], Body)> {
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("image/svg+xml"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=86400"),
            ),
        ],
        Body::from(ICON_SVG),
    ))
}

#[route(GET "/style.css")]
async fn style() -> Result<Css<&'static str>> {
    Ok(Css(CSS))
}

#[cfg(test)]
mod tests {
    use super::ICON_SVG;

    #[test]
    fn icon_is_the_master_svg() {
        assert!(ICON_SVG.trim_start().starts_with("<svg"));
        assert!(ICON_SVG.contains("</svg>"));
    }

    /// The crate-local copy must not drift from `assets/icons/flashkraft.svg`
    /// (`just icons` refreshes it). Skipped when built from the published crate.
    #[test]
    fn icon_copy_matches_the_workspace_master() {
        let master = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/icons/flashkraft.svg");
        if let Ok(m) = std::fs::read_to_string(master) {
            assert_eq!(m, ICON_SVG, "run `just icons`");
        }
    }
}
