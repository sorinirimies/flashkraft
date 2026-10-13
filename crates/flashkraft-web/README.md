# flashkraft-web

Browser front-end for [FlashKraft](https://github.com/sorinirimies/flashkraft), built with [Topcoat](https://github.com/tokio-rs/topcoat).

- Lists connected drives live (WebSocket) and shows whether a chosen image fits each one
- **Flashes one or several drives at once** with per-drive progress and cancel
- Flash history shared with the TUI and GUI, 43 themes, responsive layout, JSON API

```sh
cargo install flashkraft-web
flashkraft-web                 # http://127.0.0.1:3000
flashkraft-web --demo          # fake drives, simulated writes
flashkraft-web --read-only     # never flash from the browser
```

Live mode needs the Topcoat asset bundle (`topcoat asset bundle --package flashkraft-web --out target/debug/assets`, or `just bundle-web`); without it a static, auto-refreshing page is served.

## Safety

Flashing is accepted only when the server is bound to loopback, the `Host` header is loopback, the request carries the per-session token, the body is JSON from the same origin, every device is re-detected and re-validated on the server (no system drives, no read-only media, must fit), the exact drive count is confirmed, and no other flash is running. See the [main README](https://github.com/sorinirimies/flashkraft#why-flashing-from-a-web-page-is-safe) for details.

## API

| Route | |
|---|---|
| `GET /api/drives?image=` | drives, fit, history |
| `GET /api/flash` | current flash job and per-drive progress |
| `POST /api/flash` | start `{image, devices[], confirm}` (needs token) |
| `POST /api/flash/cancel` | stop (needs token) |
| `GET /api/history`, `/api/themes`, `/api/health` | |

MIT
