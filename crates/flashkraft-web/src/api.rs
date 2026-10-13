//! JSON and static routes.

use topcoat::{
    Result,
    context::Cx,
    router::{
        content::{Css, Json},
        query_params, route,
    },
};

use crate::{data::load, params::PageParams, style::CSS};

/// Drives (and fit for `?image=`) plus history, as JSON.
#[route(GET "/api/drives")]
async fn drives(cx: &Cx) -> Result<Json<crate::data::Snapshot>> {
    let params = query_params::<PageParams>(cx)?;
    Ok(Json(load(params.image_path()).await))
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
