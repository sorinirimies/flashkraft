//! JSON and static routes.

use topcoat::{
    Result,
    context::Cx,
    router::{
        StatusCode,
        content::{Css, Js, Json},
        query_params,
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

#[route(GET "/style.css")]
async fn style() -> Result<Css<&'static str>> {
    Ok(Css(CSS))
}
