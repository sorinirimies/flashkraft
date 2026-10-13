//! Pages: the document shell, the drives dashboard, history and about.

use topcoat::{
    Result,
    context::Cx,
    router::{page, query_params},
    runtime::{connected, shard, signal},
    view::{Child, View, component, emit, live, view},
};

use crate::{
    app::is_interactive,
    components::{brand, dashboard_body, filters, history_table, live_filters, theme_style},
    data::{REFRESH_EVERY, load},
    params::{PageParams, non_empty},
    theme,
};

/// HTML shell shared by every page.
#[component]
pub async fn document(
    interactive: bool,
    theme: usize,
    title: String,
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                if !interactive {
                    // Static fallback: no runtime, so reload every 10 s.
                    <meta http-equiv="refresh" content="10">
                }
                <meta name="flashkraft-token" content=(crate::flash::config().token.clone())>
                <meta name="flashkraft-flash" content=(if crate::flash::config().enabled { "on" } else { "off" })>
                <title>(title)</title>
                <link rel="stylesheet" href="/style.css">
                theme_style(index: theme)
                topcoat::dev::script()
                if interactive {
                    topcoat::runtime::script()
                }
            </head>
            <body><main>(child)</main></body>
        </html>
    })
}

fn saved_theme(requested: Option<&str>) -> usize {
    theme::resolve(
        requested,
        flashkraft_core::settings::saved_theme_name().as_deref(),
    )
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let params = query_params::<PageParams>(cx)?;
    let interactive = is_interactive(cx);
    let theme_index = saved_theme(params.theme.as_deref());
    let image = params.image_path().unwrap_or_default();

    // Static fallback loads once here; the live path loads inside the shard.
    let snapshot = if interactive {
        None
    } else {
        Some(load(params.image_path()).await)
    };

    Ok(view! {
        document(interactive: interactive, theme: theme_index, title: "FlashKraft — drives".to_string(),
            if let Some(s) = &snapshot {
                <header class="top">
                    brand(active: "drives".to_string())
                </header>
                <div id="flash-root"></div>
                filters(image: image.clone(), theme: theme_index)
                dashboard_body(s: s, live: false)
            } else {
                <header class="top">brand(active: "drives".to_string())</header>
                <div id="flash-root"></div>
                live_dashboard(image: image, theme: theme::names()[theme_index].to_string())
            }
            <script src="/flash.js"></script>
        )
    })
}

#[page("/history")]
async fn history_page(cx: &Cx) -> Result<impl View> {
    let params = query_params::<PageParams>(cx)?;
    let theme_index = saved_theme(params.theme.as_deref());
    let history = load(None).await.history;
    Ok(view! {
        document(interactive: false, theme: theme_index, title: "History — FlashKraft".to_string(),
            <header class="top">brand(active: "history".to_string())</header>
            <section>
                <h2>"Flash history"</h2>
                if history.is_empty() {
                    <div class="empty">"Nothing flashed yet. Successful flashes from the TUI, GUI and web show up here."</div>
                } else {
                    history_table(history: &history)
                }
            </section>
        )
    })
}

#[page("/about")]
async fn about_page(cx: &Cx) -> Result<impl View> {
    let params = query_params::<PageParams>(cx)?;
    let theme_index = saved_theme(params.theme.as_deref());
    let rows: Vec<(&str, String)> = vec![
        ("version", env!("CARGO_PKG_VERSION").to_string()),
        ("repository", env!("CARGO_PKG_REPOSITORY").to_string()),
        ("license", env!("CARGO_PKG_LICENSE").to_string()),
        ("themes", theme::names().len().to_string()),
        (
            "settings",
            flashkraft_core::settings::config_dir()
                .map(|d| d.display().to_string())
                .unwrap_or_else(|| "unknown".into()),
        ),
    ];
    Ok(view! {
        document(interactive: false, theme: theme_index, title: "About — FlashKraft".to_string(),
            <header class="top">brand(active: "about".to_string())</header>
            <section>
                <h2>"FlashKraft"</h2>
                <p class="dim">"A fast, safe way to write OS images to USB drives — terminal, desktop and web."</p>
                <table class="kv"><tbody>
                    for (k, v) in rows {
                        <tr>
                            <th>(k)</th>
                            <td>
                                if v.starts_with("http") {
                                    <a href=(v.clone()) target="_blank" rel="noopener">(v.clone())</a>
                                } else {
                                    (v.clone())
                                }
                            </td>
                        </tr>
                    }
                </tbody></table>
                <p class="dim">
                    if crate::flash::config().enabled {
                        "Flashing is enabled on this server (loopback only). Each flash needs a per-session token, re-checks every drive on the server, and asks for confirmation."
                    } else {
                        "Flashing is disabled on this server: it is read-only."
                    }
                </p>
            </section>
        )
    })
}

/// The live dashboard. The image path and theme are signals; because this
/// shard reads them on the server, changing one re-renders only this shard.
/// While a connection is open the drive list is re-read every few seconds.
#[shard]
async fn live_dashboard(cx: &Cx, image: String, theme: String) -> Result<impl View> {
    let image = signal(cx, move || image);
    let theme = signal(cx, move || theme);

    // Signal values come back from the browser: untrusted. Unknown theme → saved.
    let theme_index = saved_theme(Some(&theme.get()));
    let image_path = non_empty(Some(&image.get()));

    Ok(view! {
        theme_style(index: theme_index)
        live_filters(image: &image, theme: &theme)
        (live! {
            loop {
                let s = load(image_path.clone()).await;
                let is_connected = connected(cx);
                let token = emit! { dashboard_body(s: &s, live: is_connected) }?;
                if !is_connected {
                    break Ok(token);
                }
                tokio::time::sleep(REFRESH_EVERY).await;
            }
        })
    })
}
