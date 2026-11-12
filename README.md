# 🌸 anirel — your MyAnimeList relation graph, in the terminal

> **Find the related anime you haven't watched yet.**

[![Latest Release](https://img.shields.io/github/v/release/Praveensenpai/anirel?style=for-the-badge&color=blue)](https://github.com/Praveensenpai/anirel/releases)
[![Rust](https://img.shields.io/badge/Rust-DEA584?style=for-the-badge&logo=rust&logoColor=black)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-89b4fa?style=for-the-badge)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux%20x86__64-FCC624?style=for-the-badge&logo=linux&logoColor=black)](https://github.com/Praveensenpai/anirel)

> [!TIP]
> **Lazy by default.** Nothing is fetched until you press `L`. Your library loads in seconds; the relation graph comes only for the title you actually ask about.

```text
anirel · Praveen20 ─ 1501 titles
▸ Library 1501   Missing 5
╭ MY LIBRARY ─────────╮┌ YURU CAMP△ ───────────┐┌ POSTER ───────────┐
│ ▌ Laid-Back Camp  ✓10││ 6/10 owned · 4 missing││ ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀    │
│   Season 2          ✓││ ✓ Laid-Back Camp     ││ ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀    │
│   Season 3          ✓││ ◌ ROOM CAMP          ││ Laid-Back Camp    │
│   …                ││   missing             ││ ▰▰▰▰▰▰▰▰▰▰▰▱  6/10    │
├─────────────────────┘└────────────────────────┘└───────────────────┘
  L load · o open · / filter · ? help · q quit
```

---

## 🚀 Install

### 🪄 One-liner

```bash
curl -fsSL https://raw.githubusercontent.com/Praveensenpai/anirel/main/install.sh | bash
```

Downloads the latest release, verifies its SHA-256, installs to `~/.local/bin/anirel`.

### From source

```bash
git clone https://github.com/Praveensenpai/anirel
cd anirel && cargo install --path .
```

---

## ✨ Features

- **⚡ Lazy loading** — the relation graph is fetched on demand (`L`), never all at once
- **🎨 Poster card** — real cover art rendered as half-block pixels, with a completion bar
- **🔀 Dive anywhere** — press `L` on a side story; the pane follows its subtree
- **🧭 Release-date watch order** — sorted by air date, relation edges only label the entries
- **🔎 Both titles matched** — search finds entries by original *and* English title
- **📦 Stable `--json`** — every command speaks machine-readable output

---

## 🗝️ Keys

| Key | Action |
|---|---|
| `L` | load the franchise of the highlighted title |
| `o` / `Enter` | open it on MyAnimeList |
| `/` | filter (both titles) |
| `1` / `2` | library / missing view |
| `tab` | switch pane focus |
| `a` | full scan, with progress |
| `q` | quit |

---

## 💻 Usage

```bash
anirel                      # open the TUI
anirel related "Yuru Camp"  # everything related to one title
anirel gaps                 # every missing entry in your library
anirel --json gaps | jq .total_missing
```

In Yuru Camp's case: **6 of 10 owned** — ROOM CAMP, ROOM CAMP: Saunas and Grub, Season 2 Specials and Season 4 are the gaps.

---

## 📜 License

[MIT](LICENSE) © [Praveensenpai](https://github.com/Praveensenpai)
