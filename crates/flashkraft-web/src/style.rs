//! Stylesheet served at `/style.css`.
//!
//! Same visual language as the TokenBurn apps (and the FlashKraft TUI/GUI):
//! dark panels, 1 px borders, accent titles, pill chips. Colours are CSS
//! custom properties so every one of the 43 themes restyles the page.
//! Breakpoints match the TUI: **wide** ≥ 980 px · **medium** ≥ 700 px · **narrow** below.

pub const CSS: &str = r#"
:root { color-scheme: dark; --bg:#12121a; --panel:#1b1d29; --line:#2b3042; --fg:#e6e9f5; --dim:#8b92aa; --accent:#50c8ff; --ok:#50dc78; --warn:#ffc832; --err:#ff5050; --fg2:#ff641e; --sel:#28303f; }
* { box-sizing: border-box; }
body { margin:0; background:var(--bg); color:var(--fg); font:14px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif; }
main { max-width:1200px; margin:0 auto; padding:24px; display:flex; flex-direction:column; gap:20px; }
header.top { display:flex; flex-wrap:wrap; align-items:center; gap:16px; justify-content:space-between; }
h1 { margin:0; font-size:22px; color:var(--fg); letter-spacing:.01em; }
h1 .bolt, h1 .kraft { color:var(--accent); }
h1 .kraft { color:var(--fg); }
a.logo { color:inherit; text-decoration:none; }
.steps { display:flex; align-items:stretch; gap:10px; }
.steps .arrow { align-self:center; color:var(--dim); }
.step { flex:1; display:flex; gap:12px; align-items:center; background:var(--panel); border:1px solid var(--line); border-radius:12px; padding:12px 16px; color:var(--dim); }
.step b { display:block; color:var(--fg); }
.step .sub { display:block; font-size:12px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.step .n { width:28px; height:28px; border-radius:50%; display:grid; place-items:center; background:var(--sel); font-weight:700; flex:none; }
.step.done { border-color:var(--ok); }
.step.done .n { background:var(--ok); color:var(--bg); }
.step.ready { border-color:var(--accent); background:var(--sel); }
.step.ready .n { background:var(--accent); color:var(--bg); }
.brand { display:flex; flex-wrap:wrap; align-items:center; gap:8px 18px; }
.ver { color:var(--dim); font-weight:400; font-size:12px; margin-left:4px; }
.dim, .hint { color:var(--dim); }
.mono { font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace; font-size:12.5px; }
code { background:var(--sel); padding:1px 6px; border-radius:5px; font-size:12.5px; }
.tabs { display:flex; gap:6px; }
.tabs .tab { padding:4px 14px; border:1px solid var(--line); border-radius:999px; text-decoration:none; color:var(--dim); background:var(--panel); }
.tabs .tab.active { border-color:var(--accent); color:var(--bg); background:var(--accent); font-weight:600; }
table.kv { width:auto; }
table.kv th { text-align:left; color:var(--dim); font-weight:400; padding:5px 18px 5px 10px; white-space:nowrap; background:transparent; }
h2 { margin:0 0 8px; font-size:13px; color:var(--dim); text-transform:uppercase; letter-spacing:.08em; }
.filters { display:flex; flex-wrap:wrap; gap:10px; align-items:center; }
.filters label { display:flex; align-items:center; gap:8px; color:var(--dim); }
.filters label.grow { flex:1 1 320px; }
.filters label.grow input { flex:1; min-width:0; }
select, button, input { background:var(--panel); color:var(--fg); border:1px solid var(--line); border-radius:8px; padding:7px 12px; font:inherit; }
input:focus, select:focus { outline:none; border-color:var(--accent); }
button { cursor:pointer; background:var(--accent); color:var(--bg); border-color:var(--accent); font-weight:600; }
.cards { display:grid; grid-template-columns:repeat(auto-fit,minmax(150px,1fr)); gap:10px; }
.card { background:var(--panel); border:1px solid var(--line); border-radius:10px; padding:12px 14px; }
.card .label { color:var(--dim); font-size:12px; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.card .value { font-size:24px; font-variant-numeric:tabular-nums; }
.card.ok .value { color:var(--ok); }
.card.img { border-color:var(--accent); }
.card.img .value { color:var(--accent); }
table { width:100%; border-collapse:collapse; background:var(--panel); border:1px solid var(--line); border-radius:10px; overflow:hidden; }
th, td { padding:10px 14px; text-align:right; font-variant-numeric:tabular-nums; border-bottom:1px solid var(--line); vertical-align:middle; }
th { color:var(--accent); font-weight:600; background:var(--sel); }
th:first-child, td:first-child, td.left, th.left { text-align:left; }
tbody tr:last-child td { border-bottom:none; }
tbody tr:hover td { background:var(--sel); }
tr.blocked td { opacity:.6; }
td .name { font-weight:600; }
td .sub { color:var(--dim); font-size:12px; }
td.when { color:var(--dim); white-space:nowrap; }
.note { font-size:12px; margin-top:3px; }
.note.error { color:var(--err); }
.note.warning { color:var(--warn); }
.chip { display:inline-block; padding:1px 10px; border-radius:999px; border:1px solid var(--line); font-size:12px; font-weight:600; }
.chip.ok { color:var(--bg); background:var(--ok); border-color:var(--ok); }
.chip.warn { color:var(--bg); background:var(--warn); border-color:var(--warn); }
.chip.err { color:var(--bg); background:var(--err); border-color:var(--err); }
.chip.unknown { color:var(--dim); }
.bar { width:140px; height:8px; background:var(--line); border-radius:4px; overflow:hidden; display:inline-block; vertical-align:middle; }
.bar > span { display:block; height:100%; background:var(--accent); border-radius:4px; transition:width .45s ease; }
.bar.fit > span { background:var(--ok); }
.bar.over > span { background:var(--err); }
.pct { color:var(--dim); margin-left:8px; font-size:12px; }
.empty, .warn { padding:14px 16px; background:var(--panel); border:1px solid var(--line); border-radius:10px; color:var(--dim); }
.warn { color:var(--warn); border-color:var(--warn); }
.badge { padding:2px 10px; border-radius:999px; border:1px solid var(--line); }
.badge.live { color:var(--ok); border-color:var(--ok); }
footer { color:var(--dim); font-size:12px; }
a { color:var(--accent); }
html { -webkit-text-size-adjust:100%; }
.table-wrap { overflow-x:auto; border-radius:10px; }
.table-wrap table { min-width:100%; }
@media (prefers-reduced-motion: reduce) { .bar > span { transition:none; } }

/* medium: below 980 px — secondary columns drop out */
@media (max-width:979px) {
  .opt1 { display:none; }
  .cards { grid-template-columns:repeat(3, minmax(0,1fr)); }
}

/* narrow: below 700 px — compact header and controls, essential columns only */
@media (max-width:699px) {
  main { padding:12px 10px; gap:14px; }
  header.top { flex-direction:column; align-items:stretch; gap:10px; }
  h1 { font-size:20px; }
  .filters { display:grid; grid-template-columns:1fr; gap:8px; }
  .filters label { flex-direction:column; align-items:stretch; gap:2px; font-size:12px; }
  .cards { grid-template-columns:repeat(2, minmax(0,1fr)); }
  .card .value { font-size:19px; }
  .card.img { grid-column:1 / -1; }
  .opt2 { display:none; }
  .steps { flex-direction:column; }
  .steps .arrow { display:none; }
  th, td { padding:8px 9px; }
  footer { word-break:break-word; }
}
"#;

#[cfg(test)]
mod tests {
    use super::CSS;

    #[test]
    fn the_media_queries_use_the_tui_breakpoints() {
        assert!(CSS.contains("@media (max-width:979px)"));
        assert!(CSS.contains("@media (max-width:699px)"));
    }

    #[test]
    fn narrow_hides_secondary_columns_and_medium_hides_opt1() {
        let narrow = CSS.split("@media (max-width:699px)").nth(1).unwrap();
        assert!(narrow.contains(".opt2 { display:none; }"));
        let medium = CSS
            .split("@media (max-width:979px)")
            .nth(1)
            .unwrap()
            .split("/* narrow")
            .next()
            .unwrap();
        assert!(medium.contains(".opt1 { display:none; }"));
    }

    #[test]
    fn wide_screens_never_hide_anything() {
        let before = CSS.split("/* medium").next().unwrap();
        assert!(!before.contains(".opt1 { display:none"));
        assert!(!before.contains(".opt2 { display:none"));
    }

    #[test]
    fn every_themed_variable_has_a_fallback() {
        for v in [
            "--bg:",
            "--panel:",
            "--line:",
            "--fg:",
            "--dim:",
            "--accent:",
            "--ok:",
            "--warn:",
            "--err:",
            "--sel:",
        ] {
            assert!(CSS.contains(v), "{v}");
        }
    }
}
