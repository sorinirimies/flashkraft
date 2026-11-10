# Changelog

All notable changes to this project are documented here. It is generated from
[Conventional Commits](https://www.conventionalcommits.org) by
[git-cliff](https://git-cliff.org) — do not edit it by hand.

## [2.5.0](https://github.com/sorinirimies/flashkraft/compare/v2.4.0...v2.5.0) — 2026-11-10

### ✨ Features

- **gui:** Redesign, About/Settings/zoom, resizable window, new app icon ([`4ef2e0c`](https://github.com/sorinirimies/flashkraft/commit/4ef2e0ce84b0757ce6b697c7adbed6d64c8eb40d))

## [2.4.0](https://github.com/sorinirimies/flashkraft/compare/v2.3.7...v2.4.0) — 2026-11-07

### ✨ Features

- New app icon for macOS, Windows and Linux ([`83a6f21`](https://github.com/sorinirimies/flashkraft/commit/83a6f21f4c34f48fdcf33ccf7b4b639e330da53f))

### 📦 Dependencies

- Nightly cargo update 2026-11-07 ([`188a3fc`](https://github.com/sorinirimies/flashkraft/commit/188a3fc0b58aa7375ea10ccb3e9b151550b4d402))

### 🔄 CI and release

- **gitea:** Check the MSRV on Gitea too, with the same command as GitHub ([`9091879`](https://github.com/sorinirimies/flashkraft/commit/9091879229d3fd7fbcb8ea3de2f882c06f816c20))
- **release:** Each install line in the release notes says which binary it is (GUI, TUI, web) ([`43fce49`](https://github.com/sorinirimies/flashkraft/commit/43fce4931db2e96561a35d88364064be30cb79c8))
- **packages:** .deb and .rpm package names are valid Debian names (no underscores) ([`ec70743`](https://github.com/sorinirimies/flashkraft/commit/ec707433db24fe2a0cd3fc7320f288628febe065))

## [2.3.7](https://github.com/sorinirimies/flashkraft/compare/v2.3.6...v2.3.7) — 2026-11-06

### 🔄 CI and release

- **packages:** AppImages use the downloaded appimagetool; which also matched the script's own command ([`387fad4`](https://github.com/sorinirimies/flashkraft/commit/387fad4b77f351d533be09eddd2d2128098718e5))

## [2.3.6](https://github.com/sorinirimies/flashkraft/compare/v2.3.5...v2.3.6) — 2026-11-06

### 📦 Dependencies

- Nightly cargo update 2026-11-06 ([`10734d7`](https://github.com/sorinirimies/flashkraft/commit/10734d77217f1af8886752470ba869fe46c608ec))

### 🔄 CI and release

- **packages:** Alien gets an absolute .deb path, so the .rpm build finds it ([`1b2180e`](https://github.com/sorinirimies/flashkraft/commit/1b2180e9b1de3966d82396800ea420c7297f01a9))

## [2.3.4](https://github.com/sorinirimies/flashkraft/compare/v2.3.3...v2.3.4) — 2026-11-05

### 🐛 Bug fixes

- **ci:** The nightly downgrade guard compares within a semver series ([`e258083`](https://github.com/sorinirimies/flashkraft/commit/e258083284524b3a99f9fc5102c8b2d545399cf4))
- **gui:** Name the window on Wayland and X11 (app_id = desktop id) so desktops show its icon ([`b520794`](https://github.com/sorinirimies/flashkraft/commit/b520794d634e7e2be0a147a3cc52da3e7c749a3c))

### ♻️ Refactor

- Edition 2024, MSRV 1.90 and the shared lints; demo::enable instead of set_var; unsafe only in the FFI functions, each with a reason; tests that build on Windows; drop unused dependencies ([`d02f032`](https://github.com/sorinirimies/flashkraft/commit/d02f032e2de88bd635f17e754e2d468159968565))

### 📦 Dependencies

- Update the yanked async-executor and clipboard-win; allow NCSA and CDLA-Permissive-2.0; list the Iced stack's unmaintained crates, pin-utils included, in deny.toml ([`d4e3213`](https://github.com/sorinirimies/flashkraft/commit/d4e321311f27cab09fded1c116bf440f06b82572))

### 🔄 CI and release

- **wayland:** Start the desktop app on headless GNOME (mutter) - weston has no seat, which gpui needs - inside one session bus, as nobody when root ([`935ca7b`](https://github.com/sorinirimies/flashkraft/commit/935ca7b1420a0fbee609776d594821acdcdbf5ef))
- Download archives to a file so curl retries a dropped transfer, instead of piping into tar ([`a3d68e1`](https://github.com/sorinirimies/flashkraft/commit/a3d68e142c617a3e2b8f57a546f40a1cb7b3bd3d))
- **wayland:** Run the smoke as nobody when the job runs as root; FlashKraft refuses an interactive start as root ([`271eefd`](https://github.com/sorinirimies/flashkraft/commit/271eefdaa9b4d5cf5f3c005b1b87f1c3b3b8d171))
- Start the desktop app on a headless Wayland compositor; StartupWMClass follows the desktop id ([`120f102`](https://github.com/sorinirimies/flashkraft/commit/120f102ca2545b23d56da78e6eff4950359cbb4a))
- The Dependabot sweep asks for a rebase on conflicts and keeps going after a failed merge ([`8951a12`](https://github.com/sorinirimies/flashkraft/commit/8951a12a1e901fd0b73b05387ce283b5705dcac5))
- Merge green Dependabot PRs, majors included, with a nightly sweep; nightly commits skip ignored lockfiles ([`a539209`](https://github.com/sorinirimies/flashkraft/commit/a539209a17544b2ee63618f509c7df6e76beec9f))
- Create-rust-project --desktop packaging (deb/rpm, AppImage, DMG, installer and AUR from project.nu); demo and screenshot scripts in Nushell; secrets scan, unused-dependency gate ([`b797b4d`](https://github.com/sorinirimies/flashkraft/commit/b797b4d3fd7dc5c12f948f529f67bb803cc5b947))

## [2.3.3](https://github.com/sorinirimies/flashkraft/compare/v2.3.2...v2.3.3) — 2026-11-02

### ♻️ Refactor

- **ci:** Package macOS DMG and Windows installer with tested Nushell scripts ([`4b55678`](https://github.com/sorinirimies/flashkraft/commit/4b55678b7d09ad7c07dabcd9ac0c80f7cf5cc560))

### 📚 Documentation

- **ci:** Point NSIS header at the Nushell script ([`1f92cb3`](https://github.com/sorinirimies/flashkraft/commit/1f92cb38c63f5c6dc76665406e25ecb1b471eed9))
- Drop the per-repo agent rules skill; the shared skills cover it ([`e6ab08b`](https://github.com/sorinirimies/flashkraft/commit/e6ab08bff72908ec8da8c2476f538ebae3c460b3))
- This repo's agent rules as a project skill in .agents/skills ([`ee6f816`](https://github.com/sorinirimies/flashkraft/commit/ee6f816bb0f1a14929b8fc464a2203d31e99ea08))

### 🔄 CI and release

- **release:** Windows NSIS setup.exe installer ([`5256859`](https://github.com/sorinirimies/flashkraft/commit/5256859d7a6e57bb6c633932f90ef14050f0de9b))
- **release:** Ship macOS universal DMGflashkraft ([`fa13256`](https://github.com/sorinirimies/flashkraft/commit/fa132560edf7c170dc8d842d0dc1072ff3935202))

### 🔧 Chores

- Ignore .codegraph index ([`38e5ed9`](https://github.com/sorinirimies/flashkraft/commit/38e5ed92ca3c6639187a6d1ad1dae9ac2a8d4393))

## [2.3.1](https://github.com/sorinirimies/flashkraft/compare/v2.3.0...v2.3.1) — 2026-11-01

### 📚 Documentation

- Move agent rules from .zed/rules.md to AGENTS.md ([`84b8b84`](https://github.com/sorinirimies/flashkraft/commit/84b8b843e5588820f41eaedaa7767a214fc6499f))

### 🔄 CI and release

- **fuzz:** Fall back to an older nightly when the latest cannot build ([`3802d47`](https://github.com/sorinirimies/flashkraft/commit/3802d4743eaec1ac811cf0bc4d106e2b1e7ad815))

### 🔧 Chores

- Remove AGENTS.md ([`42284dc`](https://github.com/sorinirimies/flashkraft/commit/42284dc26d4e7bb29bb166913c7f69934de94de1))

## [2.3.0](https://github.com/sorinirimies/flashkraft/compare/v2.2.3...v2.3.0) — 2026-10-29

### ♻️ Refactor

- **tui:** Drop TuiPalette; UI reads tui-nuances Theme directly (fixes Gruvbox Dark / Atom One Dark lookups) ([`9157875`](https://github.com/sorinirimies/flashkraft/commit/9157875b1926828a6d056d6e3e5d4ae4cc7d5523))

### 📦 Dependencies

- Tui-file-explorer 4.0 (tui-nuances-only themes) ([`6e90a1e`](https://github.com/sorinirimies/flashkraft/commit/6e90a1ecb8813b2cc6b01e538b627a9383ad48b1))
- Bump tui-nuances to 0.1.10 ([`c59177b`](https://github.com/sorinirimies/flashkraft/commit/c59177b03a4d01af0972862faa958b91fb71bc43))

## [2.2.3](https://github.com/sorinirimies/flashkraft/compare/v2.2.2...v2.2.3) — 2026-10-26

### ✨ Features

- **tui:** Derive app palettes from tui-nuances ([`a2b53a9`](https://github.com/sorinirimies/flashkraft/commit/a2b53a9de3cb0261d3f1f25e28e40a57377b1c9b))

### 🐛 Bug fixes

- **fuzz:** Tui_keys only emits non-mutating keys so the explorer can't delete the corpus dir ([`49093cf`](https://github.com/sorinirimies/flashkraft/commit/49093cf68b3fc54dc73cf12d54b387ea1b2df0b5))

### 📦 Dependencies

- Bump rustls-webpki and webpki-roots for RUSTSEC-2026-0382/0381 ([`75e9337`](https://github.com/sorinirimies/flashkraft/commit/75e93375c88caf4db1d90aee0ad8135a36e24593))
- Cargo update ([`88d8368`](https://github.com/sorinirimies/flashkraft/commit/88d83682edd364ffcb187767e549743dcd1b2205))
- Bump rustls to 0.23.46 (RUSTSEC-2026-0348/0349 and others) ([`3b00e42`](https://github.com/sorinirimies/flashkraft/commit/3b00e426daa636dd428d892ab96cda40081ed576))

### 🔄 CI and release

- Ignore unmaintained zbus/zvariant advisories in cargo-deny ([`344d8ab`](https://github.com/sorinirimies/flashkraft/commit/344d8ab5ab51e2eb0bf1e3844f17dfa0fbd8c0d6))
- Nightly merges lockfile-only updates; bump + release only when a Cargo.toml requirement changes ([`f545c82`](https://github.com/sorinirimies/flashkraft/commit/f545c8298d91646bfe68269b5ed7b0925ff74ce3))
- Prebuilt Nushell, cargo-deny + cargo-audit security workflow ([`c606b59`](https://github.com/sorinirimies/flashkraft/commit/c606b59fd554eddc8801a67fa28d1c67f49f91ca))

### 🔧 Chores

- **justfile:** Push/pull current branch instead of hardcoded main ([`06415a0`](https://github.com/sorinirimies/flashkraft/commit/06415a0c330c085cb6f46a8fb838c75959ed28b5))
- **justfile:** Skip unconfigured remotes in *-all recipes ([`f61652b`](https://github.com/sorinirimies/flashkraft/commit/f61652bfa923bc7690d639b3240018f4e83cbf1b))
- **fuzz:** Bump rustls in fuzz lockfile to 0.23.46 (fixes cargo audit) ([`59bc948`](https://github.com/sorinirimies/flashkraft/commit/59bc94815ab8deb07b03855e27134eeab55be307))

## [2.2.2](https://github.com/sorinirimies/flashkraft/compare/v2.2.1...v2.2.2) — 2026-10-14

### ✨ Features

- **gui,tui,web:** Responsive layouts across all front-ends ([`2d90d09`](https://github.com/sorinirimies/flashkraft/commit/2d90d0911e9cf502a0668773e224dc2e6fcea281))

### 🧪 Tests

- **web:** End-to-end guard test for browse/flash endpoints; web_browse fuzz target ([`6c6c09c`](https://github.com/sorinirimies/flashkraft/commit/6c6c09ca5b68fded0dec245a5dbe447a0cd45fc8))

### 🔄 CI and release

- **fuzz:** Shellcheck-clean loop, don't let fuzzers read the target list ([`10a4ef4`](https://github.com/sorinirimies/flashkraft/commit/10a4ef423c1f24e175052f0a4cf480133e75513e))
- **fuzz:** Run on release tags, cancel superseded runs, keep a per-target corpus ([`599ddfb`](https://github.com/sorinirimies/flashkraft/commit/599ddfb537e5d3b36c4ff14da23495c8b5330930))

## [2.2.1](https://github.com/sorinirimies/flashkraft/compare/v2.2.0...v2.2.1) — 2026-10-13

### ✨ Features

- **web:** Server-side file browser to choose the image (Choose image…) ([`5d94a76`](https://github.com/sorinirimies/flashkraft/commit/5d94a76550298d85ba8e12a4fdbe366c3b517bc8))

### 🐛 Bug fixes

- **web:** Reset dialog style after browsing ([`7194195`](https://github.com/sorinirimies/flashkraft/commit/71941957b480ffad787885f2dab9653f4c9f3039))
- **web:** Static mode no longer reloads away the drive selection ([`f1a58c4`](https://github.com/sorinirimies/flashkraft/commit/f1a58c4eafa8e75a00dda66cb06af9e9ecdcd94b))
- **fuzz:** Stop tui_keys from emitting fs-mutating keys ([`bfaedd6`](https://github.com/sorinirimies/flashkraft/commit/bfaedd61c161247bc46e76453daf770e7aaebc7c))

## [2.2.0](https://github.com/sorinirimies/flashkraft/compare/v2.1.0...v2.2.0) — 2026-10-13

### 📚 Documentation

- Web --read-only in quick-start, settings.rs note ([`c93c244`](https://github.com/sorinirimies/flashkraft/commit/c93c24470ba230478b1a3f7822fda72d548947d2))

### 🧪 Tests

- **fuzz:** Web guards, TUI key sequences and GUI reducer; fuzz CI installs GUI libs ([`1ef989b`](https://github.com/sorinirimies/flashkraft/commit/1ef989b01482ca251323c3effb9a95a93222ee56))

### 📦 Build

- Rust 1.98 requirement only for flashkraft-web ([`b7ab24f`](https://github.com/sorinirimies/flashkraft/commit/b7ab24f1fdc351b9febd6d9699361ce008dc9c89))
- Require Rust 1.98+ workspace-wide ([`aece8dd`](https://github.com/sorinirimies/flashkraft/commit/aece8ddc1311fd24778063a92f7737ee558afc5b))

### 🔄 CI and release

- Publish flashkraft-web in release workflows and just publish ([`0884ea0`](https://github.com/sorinirimies/flashkraft/commit/0884ea0bbaa03e4fa02be8422354ebb809056f20))
- Split fuzz into own workflow; add audit & fuzz badges to README ([`ade3008`](https://github.com/sorinirimies/flashkraft/commit/ade30084c0271da539b1afdd8ff080c2778c8840))

## [2.0.2](https://github.com/sorinirimies/flashkraft/compare/v2.0.1...v2.0.2) — 2026-10-13

### ✨ Features

- **web:** Flash one or many drives from the browser, with layered safety guards ([`dabc652`](https://github.com/sorinirimies/flashkraft/commit/dabc652148248e6b1f18b8229e8e610ed3d36b24))

## [2.0.0](https://github.com/sorinirimies/flashkraft/compare/v1.5.1...v2.0.0) — 2026-10-13

### ✨ Features

- Web dashboard, multi-drive flashing, shared flash worker, TUI/GUI redesign ([`d8b5ccb`](https://github.com/sorinirimies/flashkraft/commit/d8b5ccb79d95b59a47a05fda47cce0b9423bf133))

### 🐛 Bug fixes

- **ci:** Gate deps-update on Cargo.lock only, add permissions, make Gitea commit-only ([`ce770ff`](https://github.com/sorinirimies/flashkraft/commit/ce770ffe41f04927b2a4dbdb1cf0eb887115ae20))

### 🧪 Tests

- Nightly fuzzing (settings, compatibility, version compare, device dedupe) ([`bf2a0df`](https://github.com/sorinirimies/flashkraft/commit/bf2a0df2a05372438491752b0099e38f1ac62111))

### 📦 Dependencies

- Bump quinn-proto 0.11.19 and rustls 0.23.45 (RUSTSEC-2026-0338/0340/0343/0285) ([`c80f91d`](https://github.com/sorinirimies/flashkraft/commit/c80f91de46fca3d22564d5ccfb5bdf0a0a342e37))

### 🔄 CI and release

- **security:** Add cargo-audit workflow and just audit recipe ([`ecb82dd`](https://github.com/sorinirimies/flashkraft/commit/ecb82dd9ef0b91536a758f47e69f6ca03c593e65))

### 🔧 Chores

- **just:** Setup-gitea-<instance> remote recipes ([`622fd47`](https://github.com/sorinirimies/flashkraft/commit/622fd4735a653cd8943b119bc4e084a8c2d43f06))

## [1.5.1](https://github.com/sorinirimies/flashkraft/compare/v1.5.0...v1.5.1) — 2026-09-11

### ✨ Features

- Check crates.io for newer flashkraft releases, banner in GUI + TUI ([`7078629`](https://github.com/sorinirimies/flashkraft/commit/7078629d80550e42c48f1b067c580b1d0bc1bbff))
- Record flash history in persisted settings ([`fb91446`](https://github.com/sorinirimies/flashkraft/commit/fb91446659967f406224f5acc4342953e4030f9f))
- **tui:** Remember last used image directory ([`ce86cd8`](https://github.com/sorinirimies/flashkraft/commit/ce86cd800afeb0618e60a83f46ada440d7ff31b5))
- **gui:** Remember last used image directory ([`25c0a76`](https://github.com/sorinirimies/flashkraft/commit/25c0a765348691eed7e770cdcbacdea47e9f4f1e))

### 🐛 Bug fixes

- **tui:** Isolate TuiStorage from real config dir under cfg(test) ([`7f39324`](https://github.com/sorinirimies/flashkraft/commit/7f39324b3f97e951403898eac0431d8c7c592bf3))
- **docs:** Remove private intra-doc link, add doc-check to release pipeline ([`b8479bc`](https://github.com/sorinirimies/flashkraft/commit/b8479bcec9587c6b9d3474c2eff3d6671c8aef47))
- **ci:** Only bump version in nightly deps workflow on real upgrades ([`80c452e`](https://github.com/sorinirimies/flashkraft/commit/80c452e6be7652cb40b661530dc2bd250a0dc2da))

## [1.5.0](https://github.com/sorinirimies/flashkraft/compare/v1.4.0...v1.5.0) — 2026-09-10

### ✨ Features

- **gui:** Flash raw devices without a setuid install ([`ef587b6`](https://github.com/sorinirimies/flashkraft/commit/ef587b62dd36fa8d9cafc1f190924901c674a754))

## [1.4.0](https://github.com/sorinirimies/flashkraft/compare/v1.3.7...v1.4.0) — 2026-09-10

### ✨ Features

- **tui:** Make the Flashing screen's Log panel focusable/scrollable, and make its layout responsive ([`59be025`](https://github.com/sorinirimies/flashkraft/commit/59be02524f275efadba3c19275376a73b9c84d20))

## [1.3.7](https://github.com/sorinirimies/flashkraft/compare/v1.3.6...v1.3.7) — 2026-09-10

### ✨ Features

- Restore transparent sudo/pkexec escalation for the TUI only ([`9fd2b28`](https://github.com/sorinirimies/flashkraft/commit/9fd2b28e31c4354cd80f50eb03ef2421f8b4c9e0))

## [1.3.5](https://github.com/sorinirimies/flashkraft/compare/v1.3.4...v1.3.5) — 2026-09-10

### 🐛 Bug fixes

- Correct gitea-starscream remote name in push/release justfile recipes ([`17cc48c`](https://github.com/sorinirimies/flashkraft/commit/17cc48c4c9acb6aa02c7a881403c326c11aab621))
- Defer privilege escalation to flash-time so the GUI and its native file dialogs start unprivileged ([`3930814`](https://github.com/sorinirimies/flashkraft/commit/39308147c0d0855616d83657192cd5ded93a7edb))

## [1.3.4](https://github.com/sorinirimies/flashkraft/compare/v1.3.3...v1.3.4) — 2026-09-10

### ✨ Features

- Restore automatic sudo/pkexec escalation on Linux ([`c064f53`](https://github.com/sorinirimies/flashkraft/commit/c064f53874816704cc35efa8355cf03e7ec43c1a))

### 🐛 Bug fixes

- **nix:** Support setuid-root installs for Nix users ([`8b27f4c`](https://github.com/sorinirimies/flashkraft/commit/8b27f4cbc6bbf25761b87d16f30c43e55683f390))
- **justfile:** Correct Gitea remote names (gitea, gitea_starscream) ([`ccd123c`](https://github.com/sorinirimies/flashkraft/commit/ccd123c66c5784355f2b420e75f72876ecbd3817))

### 📚 Documentation

- **justfile:** Warn that release-* recipes always bump version ([`31b7a1f`](https://github.com/sorinirimies/flashkraft/commit/31b7a1f38d1d6ad22c84c3363fa4a96169502952))

## [1.3.3](https://github.com/sorinirimies/flashkraft/compare/v1.3.2...v1.3.3) — 2026-09-10

### 📦 Dependencies

- Nightly dependency upgrade 2026-09-10 ([`87afcc1`](https://github.com/sorinirimies/flashkraft/commit/87afcc11b26c6eccf6f6d8bf7af9134d0d2295e5))

## [1.3.2](https://github.com/sorinirimies/flashkraft/compare/v1.3.1...v1.3.2) — 2026-09-09

### 📦 Dependencies

- Nightly dependency upgrade 2026-09-09 ([`93dc9cb`](https://github.com/sorinirimies/flashkraft/commit/93dc9cba65c60d19c9e0e44cf382069e9db5d227))

## [1.3.0](https://github.com/sorinirimies/flashkraft/compare/v1.2.2...v1.3.0) — 2026-09-08

### ✨ Features

- Harden flash safety and update dependencies ([`acd6a3d`](https://github.com/sorinirimies/flashkraft/commit/acd6a3d252e51ef20de8ad3850d47b4026e84e94))

### 🐛 Bug fixes

- Use configured Starscream remote ([`b1f97d6`](https://github.com/sorinirimies/flashkraft/commit/b1f97d683715473858e0f8bcc57622d17f6ffd8b))

### 📦 Dependencies

- Nightly dependency upgrade 2026-08-21 ([`095cf4c`](https://github.com/sorinirimies/flashkraft/commit/095cf4cd0e984d5621fd14e891166122660079f0))
- Nightly dependency upgrade 2026-08-20 ([`868d2b6`](https://github.com/sorinirimies/flashkraft/commit/868d2b60a2b9d7a570c05946b82a553069564c1f))
- Nightly dependency upgrade 2026-05-11 ([`999f250`](https://github.com/sorinirimies/flashkraft/commit/999f250cb655dd96ab5e9a9af240521e73e17c1c))
- Nightly dependency upgrade 2026-05-05 ([`c5010f0`](https://github.com/sorinirimies/flashkraft/commit/c5010f0e5bdc6e6599a39a6bc841f00e08d25998))

## [1.1.6](https://github.com/sorinirimies/flashkraft/compare/v1.1.4...v1.1.6) — 2026-05-04

### 📦 Dependencies

- Nightly dependency upgrade 2026-05-04 ([`e50a656`](https://github.com/sorinirimies/flashkraft/commit/e50a6567137c38e4023a534bf01c14d4181607bb))

## [1.1.4](https://github.com/sorinirimies/flashkraft/compare/v1.1.2...v1.1.4) — 2026-05-03

### ♻️ Refactor

- Align GUI/TUI architecture (core/ + ui/screens/ + ui/components/) ([`8f36b87`](https://github.com/sorinirimies/flashkraft/commit/8f36b8729e862440618a3df249bbea38845b1ac2))

### 📦 Dependencies

- Nightly dependency upgrade 2026-04-29 ([`98333b4`](https://github.com/sorinirimies/flashkraft/commit/98333b455124bea02761464ca5eede6a2b704889))
- Nightly dependency upgrade 2026-04-28 ([`767b115`](https://github.com/sorinirimies/flashkraft/commit/767b11592284d01dd63aec9c08df23dbe9d0b744))

## [0.8.9](https://github.com/sorinirimies/flashkraft/compare/v0.8.8...v0.8.9) — 2026-03-27

### ✨ Features

- Add Nix flake for reproducible builds ([`b01e28c`](https://github.com/sorinirimies/flashkraft/commit/b01e28c6bf3c44409ffd4ae12ff9dd42bfcbc754))

## [0.8.6](https://github.com/sorinirimies/flashkraft/compare/v0.8.5...v0.8.6) — 2026-03-23

### 🔧 Chores

- Nightly dependency update 2026-03-23 ([`f4acdcd`](https://github.com/sorinirimies/flashkraft/commit/f4acdcda4ea81cd1f632542551409f480b935846))
- Nightly dependency update 2026-03-22 ([`b58ac01`](https://github.com/sorinirimies/flashkraft/commit/b58ac01cf477e706d52ccdcc6ca48b9ef3e286d1))
- Nightly dependency update 2026-03-20 ([`e970ef6`](https://github.com/sorinirimies/flashkraft/commit/e970ef646a57e84819ab43cebbfc1b3fbd67e008))
- Nightly dependency update 2026-03-19 ([`54b0c31`](https://github.com/sorinirimies/flashkraft/commit/54b0c31c1e17608d2acc8c56211b1a0e8cb096d5))
- Nightly dependency update 2026-03-18 ([`a05f5ac`](https://github.com/sorinirimies/flashkraft/commit/a05f5ac8a3277da9ab49bf3bd9e23ef82418de5a))

## [0.8.3](https://github.com/sorinirimies/flashkraft/compare/v0.8.2...v0.8.3) — 2026-03-17

### 🔧 Chores

- Nightly dependency update 2026-03-17 ([`0fd30ba`](https://github.com/sorinirimies/flashkraft/commit/0fd30ba5f11dccba62614e63dd9dedc318da6032))
- Nightly dependency update 2026-03-16 ([`343c157`](https://github.com/sorinirimies/flashkraft/commit/343c157f64e5c01c965aedd7e36f3f088ad0bbfe))
- Nightly dependency update 2026-03-15 ([`5750b23`](https://github.com/sorinirimies/flashkraft/commit/5750b23c41fbfc4dd0ad6f765a5cfbfb3e77914a))
- Nightly dependency update 2026-03-14 ([`0db25a9`](https://github.com/sorinirimies/flashkraft/commit/0db25a9ea6cb66073438bc30b92ea26bfff99f20))

## [0.8.0](https://github.com/sorinirimies/flashkraft/compare/v0.7.9...v0.8.0) — 2026-03-13

### 🔧 Chores

- Nightly dependency update 2026-03-13 ([`9a0b1e1`](https://github.com/sorinirimies/flashkraft/commit/9a0b1e1363d59003e9112ce1de40d7645c40502d))
- Nightly dependency update 2026-03-12 ([`3af14ed`](https://github.com/sorinirimies/flashkraft/commit/3af14ed610b5262be09aeda23070db08051fa5e0))

## [0.7.4](https://github.com/sorinirimies/flashkraft/compare/v0.7.3...v0.7.4) — 2026-03-11

### 🔧 Chores

- Nightly dependency update 2026-03-05 ([`e20e7ea`](https://github.com/sorinirimies/flashkraft/commit/e20e7eafd13e994cfc6a292496b6361f930edfab))

## [0.7.0](https://github.com/sorinirimies/flashkraft/compare/v0.6.9...v0.7.0) — 2026-03-03

### 🔧 Chores

- Nightly dependency update 2026-03-02 ([`75744c2`](https://github.com/sorinirimies/flashkraft/commit/75744c24c74cc2549a10684a512b4d6ff2127706))

## [0.6.7](https://github.com/sorinirimies/flashkraft/compare/v0.6.6...v0.6.7) — 2026-03-02

### 🐛 Bug fixes

- Drop deprecated --token flag and make cargo publish idempotent on already-exists ([`7754bc4`](https://github.com/sorinirimies/flashkraft/commit/7754bc4a0139bc7acfd188f7ed7d504d93be9759))
- Replace invalid job-level secrets guard with env var gate in publish job ([`d200266`](https://github.com/sorinirimies/flashkraft/commit/d2002660e2ac8f09645e44375fea4a0376c09caa))

## [0.6.4](https://github.com/sorinirimies/flashkraft/compare/v0.6.3...v0.6.4) — 2026-03-01

### 🐛 Bug fixes

- Add --allow-dirty to cargo publish to allow copied README.md ([`56afcd2`](https://github.com/sorinirimies/flashkraft/commit/56afcd2984f2768f1d1eebcc2e7f8d55a4563b48))

### 🔧 Chores

- Nightly dependency update 2026-02-28 ([`ad4dff5`](https://github.com/sorinirimies/flashkraft/commit/ad4dff5696d31fc8bb93475d48393889b7c42c79))
- Nightly dependency update 2026-02-27 ([`ffb50c4`](https://github.com/sorinirimies/flashkraft/commit/ffb50c43833724e6ffd4c10301249a7bfdc5fe2e))

## [0.6.3](https://github.com/sorinirimies/flashkraft/compare/v0.6.2...v0.6.3) — 2026-02-26

### 🐛 Bug fixes

- Use --follow-tags to push branch and tag in one operation, preventing double release trigger ([`f9cc39b`](https://github.com/sorinirimies/flashkraft/commit/f9cc39b1a6c3c3788e50ee29aaf726f296fcfa28))

## [0.6.2](https://github.com/sorinirimies/flashkraft/compare/v0.6.1...v0.6.2) — 2026-02-26

### 🐛 Bug fixes

- Copy README.md into all crate dirs before publishing so crates.io shows it ([`52d28c6`](https://github.com/sorinirimies/flashkraft/commit/52d28c64dbd972cda162bb0436ddb61ac94dae12))

## [0.6.1](https://github.com/sorinirimies/flashkraft/compare/v0.6.0...v0.6.1) — 2026-02-26

### 🐛 Bug fixes

- Remove gh workflow dispatch from just release — tag push triggers it automatically, add release-retrigger as explicit fallback ([`1e81dcc`](https://github.com/sorinirimies/flashkraft/commit/1e81dcc19780837a769b7062b4a27931cbfaa031))

## [0.6.0](https://github.com/sorinirimies/flashkraft/compare/v0.5.9...v0.6.0) — 2026-02-26

### 🐛 Bug fixes

- Use cargo info instead of cargo search for reliable already-published check ([`896b949`](https://github.com/sorinirimies/flashkraft/commit/896b9492b17f3381828b99a1e2ac400ad04564d4))

## [0.5.9](https://github.com/sorinirimies/flashkraft/compare/v0.5.8...v0.5.9) — 2026-02-26

### 🐛 Bug fixes

- Copy README.md into crates/flashkraft-gui before publishing to crates.io ([`25ab192`](https://github.com/sorinirimies/flashkraft/commit/25ab1926b1c15c365b70bd343c9356d6e3769a04))

## [0.5.8](https://github.com/sorinirimies/flashkraft/compare/v0.5.7...v0.5.8) — 2026-02-26

### 🐛 Bug fixes

- Replace all -p flashkraft-gui references with -p flashkraft across workflows and justfile ([`4797747`](https://github.com/sorinirimies/flashkraft/commit/4797747806d03d4996e0ea5ebd392e1c9c30f8f4))

## [0.5.7](https://github.com/sorinirimies/flashkraft/compare/v0.5.6...v0.5.7) — 2026-02-26

### 🐛 Bug fixes

- Rename flashkraft-gui package to flashkraft for crates.io publishing ([`f6a7159`](https://github.com/sorinirimies/flashkraft/commit/f6a7159d1cb90d5564515e4f753c366520befae1))

## [0.5.6](https://github.com/sorinirimies/flashkraft/compare/v0.5.5...v0.5.6) — 2026-02-26

### 🐛 Bug fixes

- Skip publish if version already exists on crates.io ([`50e00f5`](https://github.com/sorinirimies/flashkraft/commit/50e00f5de27864702f0f1e384edcf0f078f60fa5))

## [0.5.5](https://github.com/sorinirimies/flashkraft/compare/v0.5.4...v0.5.5) — 2026-02-26

### 🐛 Bug fixes

- Silence unused variable warnings in flash_subscription.rs ([`30a2823`](https://github.com/sorinirimies/flashkraft/commit/30a2823b7b1716c36299152fb5ec048cb035691a))

### 🔄 CI and release

- Add flashkraft-gui to Windows check job — Iced supports Windows natively ([`150e079`](https://github.com/sorinirimies/flashkraft/commit/150e079d0a57574b2f494fb937510c5c615a86ed))

## [0.5.4](https://github.com/sorinirimies/flashkraft/compare/v0.5.3...v0.5.4) — 2026-02-26

### 🐛 Bug fixes

- Remove --locked from cargo test in all CI workflows ([`e2ea240`](https://github.com/sorinirimies/flashkraft/commit/e2ea240d8ba11f7a985704a50cca42749468f486))
- Restore flashkraft-core publishing — required by gui and tui on crates.io ([`b13fcea`](https://github.com/sorinirimies/flashkraft/commit/b13fcea7dd5314f6ca8d1a0750eb6a57eadc0dba))

## [0.5.2](https://github.com/sorinirimies/flashkraft/compare/v0.5.1...v0.5.2) — 2026-02-26

### ✨ Features

- Add update-deps just command and nightly dep update workflows ([`8864cfd`](https://github.com/sorinirimies/flashkraft/commit/8864cfda6647ccde0b7b3cbdb7f66c2b5190c232))

### 🐛 Bug fixes

- Gate nix::libc import behind #[cfg(unix)] for Windows compat ([`bbe194c`](https://github.com/sorinirimies/flashkraft/commit/bbe194ceb59026666195932dc2766421ea7371db))

### ♻️ Refactor

- Publish flashkraft (GUI) and flashkraft-tui, core is internal only ([`f72b23d`](https://github.com/sorinirimies/flashkraft/commit/f72b23d1c9f5d954fe2cee45a3edf36da58a4e85))
- Consolidate to single published flashkraft crate with gui/tui features ([`b7f2523`](https://github.com/sorinirimies/flashkraft/commit/b7f2523cbe207a0abcaa8dbe397ac8aecaccc1e5))

### 🔄 CI and release

- Add Windows cargo check job for core and tui ([`b3091bf`](https://github.com/sorinirimies/flashkraft/commit/b3091bf14cb0513ce2aeed8e6e3d46aac282e53a))
- Add Windows x86_64 msvc target, fix cross-platform artifact staging ([`952d018`](https://github.com/sorinirimies/flashkraft/commit/952d0184134d7c99bc57788dcfe108ddc19ec368))
- Drop aarch64 and musl targets, keep x86_64-unknown-linux-gnu only ([`3f6a31b`](https://github.com/sorinirimies/flashkraft/commit/3f6a31b0ccee130b3988dae73fdb9b968734740f))

### 🔧 Chores

- Remove unnecessary sleep between crates.io publishes ([`a782f78`](https://github.com/sorinirimies/flashkraft/commit/a782f783189b72b2ee945adbf9ae43463f423c1b))

## [0.5.1](https://github.com/sorinirimies/flashkraft/compare/v0.5.0...v0.5.1) — 2026-02-26

### 🐛 Bug fixes

- Remove secrets condition from publish job — breaks workflow_dispatch ([`7840550`](https://github.com/sorinirimies/flashkraft/commit/7840550fd73094f5c467f53c1f3a2e35f7d8f47c))

## [0.1.3](https://github.com/sorinirimies/flashkraft/releases/tag/v0.1.3) — 2025-10-24


