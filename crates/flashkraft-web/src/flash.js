// FlashKraft web — selection, confirmation and live progress.
// Everything user/device-derived is inserted with textContent, never innerHTML.
(() => {
  'use strict';
  const meta = (n) => document.querySelector(`meta[name="${n}"]`)?.content || '';
  const token = meta('flashkraft-token');
  const enabled = meta('flashkraft-flash') === 'on';
  if (!enabled) document.documentElement.classList.add('noflash');
  const selected = new Set();
  let dismissedFinished = false;
  let lastStatus = null;

  const el = (tag, cls, text) => {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    return e;
  };

  // ---- root ---------------------------------------------------------------
  const root = document.getElementById('flash-root');
  if (!root) return;
  const bar = el('div', 'flash-bar');
  const panel = el('section', 'flash-panel');
  panel.hidden = true;
  root.append(bar, panel);

  const dlg = el('dialog', 'flash-dialog');
  document.body.append(dlg);

  // ---- selection ----------------------------------------------------------
  const picks = () => [...document.querySelectorAll('input.pick')];
  const imagePath = () => document.querySelector('[data-image]')?.dataset.image || '';

  function syncBoxes() {
    // The live table is re-rendered by the server: re-apply what the user ticked.
    const present = new Set();
    for (const b of picks()) {
      present.add(b.value);
      if (b.checked !== selected.has(b.value)) b.checked = selected.has(b.value);
    }
    for (const v of [...selected]) if (!present.has(v)) selected.delete(v); // unplugged
    renderBar();
  }

  document.addEventListener('change', (e) => {
    const t = e.target;
    if (!(t instanceof HTMLInputElement) || !t.classList.contains('pick')) return;
    if (t.checked) selected.add(t.value); else selected.delete(t.value);
    renderBar();
  });

  function renderBar() {
    bar.replaceChildren();
    const running = lastStatus?.running;
    if (!enabled) {
      bar.append(el('span', 'dim', 'Flashing is disabled on this server (read-only, or not bound to loopback).'));
      return;
    }
    const n = selected.size;
    const btn = el('button', 'flash-go', n ? `⚡ Flash ${n} drive${n > 1 ? 's' : ''}` : '⚡ Flash');
    btn.disabled = !n || !imagePath() || running;
    btn.addEventListener('click', confirmDialog);
    bar.append(btn);
    bar.append(el('span', 'dim', !imagePath()
      ? 'Enter an image path, then tick the drives to write.'
      : running ? 'A flash is running.' : n ? 'Selected drives will be erased.' : 'Tick one or more drives.'));
  }

  // ---- confirm ------------------------------------------------------------
  function confirmDialog() {
    const devices = [...selected];
    const rows = picks().filter((b) => selected.has(b.value));
    dlg.replaceChildren();
    dlg.append(el('h3', '', `Erase ${devices.length} drive${devices.length > 1 ? 's' : ''}?`));
    dlg.append(el('p', '', `Everything on the drives below will be destroyed and replaced with ${imagePath().split('/').pop()}.`));
    const ul = el('ul', 'flash-list');
    for (const b of rows) ul.append(el('li', '', `${b.dataset.name || b.value}  ·  ${b.value}`));
    dlg.append(ul);
    const msg = el('p', 'flash-err');
    const row = el('div', 'flash-actions');
    const cancel = el('button', 'ghost', 'Cancel');
    const go = el('button', 'danger', `Erase and flash ${devices.length}`);
    cancel.addEventListener('click', () => dlg.close());
    go.addEventListener('click', async () => {
      go.disabled = true;
      msg.textContent = '';
      const r = await post('/api/flash', { image: imagePath(), devices, confirm: devices.length });
      if (r.ok) { dlg.close(); selected.clear(); dismissedFinished = false; syncBoxes(); poll(); }
      else { msg.textContent = r.error; go.disabled = false; }
    });
    row.append(cancel, go);
    dlg.append(msg, row);
    dlg.showModal();
  }

  async function post(url, body) {
    try {
      const res = await fetch(url, {
        method: 'POST',
        headers: { 'content-type': 'application/json', 'x-flashkraft-token': token },
        body: JSON.stringify(body || {}),
      });
      const j = await res.json().catch(() => ({}));
      return res.ok ? { ok: true } : { ok: false, error: j.error || `request failed (${res.status})` };
    } catch (e) {
      return { ok: false, error: String(e) };
    }
  }

  // ---- progress -----------------------------------------------------------
  const pct = (f) => `${Math.round(Math.max(0, Math.min(1, f)) * 100)}%`;

  function renderPanel(s) {
    if (!s.present || (!s.running && dismissedFinished)) { panel.hidden = true; return; }
    panel.hidden = false;
    panel.replaceChildren();
    const head = el('div', 'flash-head');
    const title = s.running
      ? `Flashing ${s.targets.length} drive${s.targets.length > 1 ? 's' : ''}… ${pct(s.progress)}`
      : s.failed ? `Finished with ${s.failed} failure${s.failed > 1 ? 's' : ''}` : `All ${s.done} drive${s.done > 1 ? 's' : ''} are ready`;
    head.append(el('h2', '', title));
    head.append(el('span', 'dim', s.running
      ? `${s.done} done · ${s.failed} failed · ${s.speed_mb_s.toFixed(1)} MB/s · ${s.image}`
      : s.image));
    const act = el('div', 'flash-actions');
    if (s.running) {
      const c = el('button', 'ghost', 'Cancel all');
      c.addEventListener('click', () => post('/api/flash/cancel'));
      act.append(c);
    } else {
      const d = el('button', 'ghost', 'Dismiss');
      d.addEventListener('click', () => { dismissedFinished = true; renderPanel(s); renderBar(); });
      act.append(d);
    }
    head.append(act);
    panel.append(head);
    for (const t of s.targets) {
      const card = el('div', `flash-target ${t.status}`);
      const top = el('div', 'flash-row');
      top.append(el('b', '', t.label), el('span', 'dim', t.status === 'failed' ? 'failed' : pct(t.progress)));
      const track = el('div', 'flash-track');
      const fill = el('div', 'flash-fill');
      fill.style.width = pct(t.status === 'done' ? 1 : t.progress);
      track.append(fill);
      const sub = el('div', 'dim flash-sub', t.error
        ? t.error
        : t.status === 'running' ? `${t.speed_mb_s.toFixed(1)} MB/s · ${t.write < 1 ? 'writing' : 'verifying'}`
        : t.status === 'queued' ? 'waiting for a free lane' : t.message);
      card.append(top, track, sub);
      panel.append(card);
    }
  }

  let timer = null;
  async function poll() {
    clearTimeout(timer);
    let delay = 3000;
    try {
      const res = await fetch('/api/flash', { cache: 'no-store' });
      if (res.ok) {
        const s = await res.json();
        const was = lastStatus?.running;
        lastStatus = s;
        if (s.running && dismissedFinished) dismissedFinished = false;
        renderPanel(s);
        if (was !== s.running) renderBar();
        delay = s.running ? 600 : 3000;
      }
    } catch (_) { /* server restarting: try again */ }
    timer = setTimeout(poll, delay);
  }

  // The server re-renders the table while a flash runs; keep the ticks.
  // Ignore our own DOM (bar/panel/dialog) or we would observe ourselves forever.
  new MutationObserver((muts) => {
    const foreign = muts.some((m) => !root.contains(m.target) && !dlg.contains(m.target) && m.target !== dlg);
    if (foreign) syncBoxes();
  }).observe(document.body, { childList: true, subtree: true });
  syncBoxes();
  poll();

  // Static mode (no Topcoat bundle): refresh the drive list, but never while
  // the user is mid-selection or a flash is being confirmed/running.
  if (meta('flashkraft-static') === 'on') {
    setInterval(() => {
      const typing = document.activeElement instanceof HTMLInputElement
        && document.activeElement.type === 'text';
      if (!selected.size && !dlg.open && !lastStatus?.running && !typing) location.reload();
    }, 10000);
  }
})();
