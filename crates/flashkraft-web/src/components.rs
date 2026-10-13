//! Server-rendered page components.

use topcoat::{
    Result,
    runtime::{Event, Signal},
    view::{View, component, view},
};

use crate::data::{DriveRow, Fit, HistoryRow, Snapshot};
use crate::theme;

/// Title with the version, plus the Drives / History / About tabs.
#[component]
pub async fn brand(active: String) -> Result<impl View> {
    let pick = |name: &str| if active == name { "tab active" } else { "tab" };
    let (c_drives, c_hist, c_about) = (pick("drives"), pick("history"), pick("about"));
    Ok(view! {
        <div class="brand">
            <h1><a href="/" class="logo"><span class="bolt">"⚡"</span> "Flash" <span class="kraft">"Kraft"</span></a> <small class="ver">(concat!("v", env!("CARGO_PKG_VERSION")))</small></h1>
            <nav class="tabs" aria-label="Pages">
                <a class=(c_drives) href="/">"Drives"</a>
                <a class=(c_hist) href="/history">"History"</a>
                <a class=(c_about) href="/about">"About"</a>
            </nav>
        </div>
    })
}

/// Three step cards (image → drive → flash) that light up as each is satisfied.
#[component]
pub async fn steps(s: &Snapshot) -> Result<impl View> {
    let has_image = s.image.is_some();
    let has_drive = has_image
        && s.drives
            .iter()
            .any(|d| matches!(d.fit, Fit::Ok | Fit::Warn));
    let c1 = if has_image { "step done" } else { "step" };
    let c2 = if has_drive { "step done" } else { "step" };
    let c3 = if has_image && has_drive {
        "step ready"
    } else {
        "step"
    };
    let t1 = s
        .image
        .as_ref()
        .map(|i| i.name.clone())
        .unwrap_or_else(|| "Choose an image".into());
    let t2 = if has_drive {
        "A drive fits".to_string()
    } else {
        "Pick a drive".to_string()
    };
    Ok(view! {
        <section class="steps" aria-label="Flash steps">
            <div class=(c1)><span class="n">"1"</span><div><b>"Select image"</b><span class="sub">(t1)</span></div></div>
            <span class="arrow">"▸"</span>
            <div class=(c2)><span class="n">"2"</span><div><b>"Select drive"</b><span class="sub">(t2)</span></div></div>
            <span class="arrow">"▸"</span>
            <div class=(c3)><span class="n">"3"</span><div><b>"Flash!"</b><span class="sub">(if has_image && has_drive { "tick drives below" } else { "tick drives, then flash" })</span></div></div>
        </section>
    })
}

/// Row of headline totals.
#[component]
pub async fn summary_cards(s: &Snapshot) -> Result<impl View> {
    let (image_value, image_label) = match &s.image {
        Some(i) => (i.size_label.clone(), format!("image · {}", i.name)),
        None => ("—".to_string(), "image".to_string()),
    };
    Ok(view! {
        <section class="cards">
            <div class="card"><div class="label">"drives"</div><div class="value">(s.drives.len())</div></div>
            <div class="card"><div class="label">"usb sticks"</div><div class="value">(s.usb_count)</div></div>
            <div class="card ok"><div class="label">"flashable"</div><div class="value">(s.usable_count)</div></div>
            <div class="card"><div class="label">"total capacity"</div><div class="value">(flashkraft_core::fmt_bytes(s.total_bytes))</div></div>
            <div class="card img"><div class="label">(image_label)</div><div class="value">(image_value)</div></div>
        </section>
    })
}

/// Theme `<style>` element for the active palette.
#[component]
pub async fn theme_style(index: usize) -> Result<impl View> {
    let css = theme::css(index);
    Ok(view! { <style>(css)</style> })
}

/// GET form that drives the query string (static mode).
#[component]
pub async fn filters(image: String, theme: usize) -> Result<impl View> {
    Ok(view! {
        <form class="filters" method="get" action="/">
            <label class="grow">"image "
                <input type="text" name="image" value=(image) placeholder="/path/to/debian.iso" spellcheck="false">
            </label>
            <label>"theme "
                <select name="theme" onchange="this.form.submit()">
                    for (i, n) in theme::names().iter().enumerate() {
                        <option value=(*n) selected=(i == theme)>(*n)</option>
                    }
                </select>
            </label>
            <button type="submit">"Check fit"</button>
        </form>
    })
}

/// Inputs bound to signals: changing one re-renders the enclosing shard.
#[component]
pub async fn live_filters(image: &Signal<String>, theme: &Signal<String>) -> Result<impl View> {
    let (img, th) = (image.get_untracked(), theme.get_untracked());
    Ok(view! {
        <div class="filters">
            <label class="grow">"image "
                <input type="text" placeholder="/path/to/debian.iso" spellcheck="false" value=(img)
                    :value=$(image.get()) @change=$(|e: Event| image.set(e.target.value))>
            </label>
            <label>"theme "
                <select :value=$(theme.get()) @change=$(|e: Event| theme.set(e.target.value))>
                    for n in theme::names() {
                        <option value=(*n) selected=(*n == th)>(*n)</option>
                    }
                </select>
            </label>
        </div>
    })
}

/// One status chip: `ready` / `check` / `blocked` / `—`.
#[component]
pub async fn fit_chip(fit: Fit) -> Result<impl View> {
    let class = format!("chip {}", fit.css());
    Ok(view! { <span class=(class)>(fit.label())</span> })
}

/// The drive table: capacity bar, USB info, and fit against the chosen image.
#[component]
pub async fn drives_table(
    drives: &[DriveRow],
    has_image: bool,
    image: String,
) -> Result<impl View> {
    Ok(view! {
        <div class="table-wrap" data-image=(image)><table>
            <thead>
                <tr>
                    <th class="pickcol"></th>
                    <th class="left">"drive"</th>
                    <th class="left opt2">"device"</th>
                    <th>"size"</th>
                    <th class="left opt2">"capacity"</th>
                    if has_image { <th class="left opt1">"image fills"</th> }
                    <th class="left">"status"</th>
                </tr>
            </thead>
            <tbody>
                for d in drives {
                    <tr class=(if d.fit == Fit::Blocked { "blocked" } else { "" })>
                        <td class="pickcol">
                            if has_image && matches!(d.fit, Fit::Ok | Fit::Warn) {
                                <input type="checkbox" class="pick" value=(d.device_path.clone()) data-name=(d.name.clone()) aria-label=(format!("Select {}", d.name))>
                            }
                        </td>
                        <td class="left">
                            <div class="name">(d.name.clone())</div>
                            if let Some(u) = &d.usb {
                                <div class="sub">"🔌 " (u.clone())</div>
                            } else {
                                <div class="sub">"internal · " (d.mount_point.clone())</div>
                            }
                        </td>
                        <td class="left opt2 mono">(d.device_path.clone())</td>
                        <td>(d.size_label.clone())</td>
                        <td class="left opt2"><span class="bar"><span style=(format!("width:{}%", d.capacity_pct))></span></span></td>
                        if has_image {
                            <td class="left opt1">
                                if let Some(p) = d.image_pct {
                                    <span class=(if p >= 100 { "bar over" } else { "bar fit" })><span style=(format!("width:{}%", p))></span></span>
                                    <span class="pct">(format!("{p}%"))</span>
                                }
                            </td>
                        }
                        <td class="left">
                            fit_chip(fit: d.fit)
                            for (kind, msg) in &d.notes {
                                <div class=(format!("note {kind}"))>(msg.clone())</div>
                            }
                        </td>
                    </tr>
                }
            </tbody>
        </table></div>
    })
}

/// Flash history table.
#[component]
pub async fn history_table(history: &[HistoryRow]) -> Result<impl View> {
    Ok(view! {
        <div class="table-wrap"><table>
            <thead>
                <tr>
                    <th class="left">"image"</th>
                    <th class="left">"drive"</th>
                    <th>"size"</th>
                    <th>"when"</th>
                </tr>
            </thead>
            <tbody>
                for h in history {
                    <tr>
                        <td class="left"><div class="name">(h.image_name.clone())</div><div class="sub mono">(h.image_path.clone())</div></td>
                        <td class="left">(h.drive_label.clone())</td>
                        <td>(h.size_label.clone())</td>
                        <td class="when">(h.ago.clone())</td>
                    </tr>
                }
            </tbody>
        </table></div>
    })
}

/// Cards and the drive table for one [`Snapshot`].
#[component]
pub async fn dashboard_body(s: &Snapshot, live: bool) -> Result<impl View> {
    Ok(view! {
        if s.demo {
            <div class="warn">"Demo mode — these are fake drives. Nothing here can be written."</div>
        }
        if let Some(e) = &s.image_error {
            <div class="warn">"⚠ " (e.clone())</div>
        }

        steps(s: s)

        summary_cards(s: s)

        <section>
            <h2>"Drives"</h2>
            if s.drives.is_empty() {
                <div class="empty">"No drives detected. Plug in a USB stick — this page refreshes itself."</div>
            } else {
                drives_table(drives: &s.drives, has_image: s.image.is_some(), image: s.image.as_ref().map(|i| i.path.clone()).unwrap_or_default())
            }
        </section>

        if s.image.is_none() && s.image_error.is_none() {
            <p class="hint">"Type an image path above to see which drives it fits on."</p>
        }

        <footer>
            if live {
                <span class="badge live">"● live"</span>
            } else {
                <span class="badge">"○ static"</span>
            }
            " updated " (s.updated_at.clone())
            " · JSON at " <a href="/api/drives">"/api/drives"</a>
        </footer>
    })
}
