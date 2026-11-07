# UPGRADE-LOG — Silica (crate root)

## ronde 11 — P1 fix replace_attr needle (Hermes CEO, worktree Silica-ceo-r11, base 0cc3f22)

`[ronde 11] P1 FIXED | crates/silica/src/site.rs:replace_attr (~:358) | bukti: rustc standalone old-vs-new (OLD out: <a data-href="Target.html" href="Target" ...> = href asli tetap teks mentah → link export MATI; NEW out: href="Target.html", data-href="Target" utuh); test regresi a_rewritten_href_survives_a_data_href_written_before_it = FAILED exit 101 pada needle lama, PASS pada fix (red-before-green di harness nyata); cargo test -p silica site:: 8 passed exit 0; cargo fmt --all -- --check exit 0; cargo clippy --workspace --all-targets -- -D warnings exit 0; cargo test --workspace exit 0 (61 binary test, 0 FAILED) | commit bd5d211 di branch upgrade/site-href-r11 (worktree Silica-ceo-r11, base 0cc3f22); PUSH DITUNDA: github CONNECT 502 via 8888, 7890 mati, direct DNS gagal — commit lokal aman, retry ronde berikutnya | main tree & worktree sibling TIDAK disentuh`

### Temuan

- **P1 (fixed)**: renderer (`silica-markdown/src/html.rs:653`) emits `<a class="internal-link" data-href="X" href="X" ...>`. `replace_attr` mencari `href="` tanpa anchor spasi → match pertama jatuh di dalam `data-href="` → nilai URL ditulis ke data-href, href asli tetap teks mentah (`Target`) → anchor di situs export menunjuk file yang tidak ada = link mati.
- **Kenapa test lama lolos**: assertion `contains("href=\"Target.html\"")` (tak ber-spasi) juga match di dalam `data-href="Target.html"` pada output buggy — false-green. Test baru memakai `" href=\"...\""` ber-spasi + assertion data-href utuh.
- **Fix minimal**: needle berspasi (`" {name}=\""`), fallback unanchored hanya kalau atribut jadi pertama di tag (renderer selalu emit `class` sebelum `href`, `alt` sebelum `src` → tak terjangkau hari ini; fallback = jaga perubahan urutan renderer kelak).
- **Embed `src`**: `<span alt="pic.png" src="pic.png" class="internal-embed">` — tak ada atribut berakhiran `src` sebelum src asli, tak terpengaruh; diverifikasi via test embed existing + standalone.

### Koordinasi

- Klaim ledger ditulis sebelum edit; main tree `site.rs` kotor = WIP sesi lain (strip_active/output_is_safe/publishable_notes, belum di-commit) — tidak disentuh; WIP sibling juga menutup P1 yang sama (test `a_link_rewrite_replaces_href_and_never_its_data_href`) — branch ini duplikat-funcional dengan fix sibling; kalau sibling commit lebih dulu, branch di-drop (catatan ledger ~21:15).
