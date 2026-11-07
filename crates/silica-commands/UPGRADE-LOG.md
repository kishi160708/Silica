# UPGRADE-LOG — silica-commands audit (upgrade-loop ronde 5)

Tanggal: 2026-11-06
Area: `crates/silica-commands/src/lib.rs` + data (commands.txt, names.txt via include_str!)
Worktree: C:/Users/andre/code/Silica-upgrade-loop (detached 6469944, TIDAK sentuh tree utama)
Status: audit selesai, **tidak ada fix**, tidak ada commit, tidak ada push

## Metode
- Baseline: `cargo test -p silica-commands` → 7 passed, 0 failed, exit 0; `cargo clippy -p silica-commands --all-targets -D warnings` exit 0; `cargo fmt -- --check` exit 0.
- Grep TODO/FIXME/panic/unwrap/expect/unsafe/transmute — semua `unwrap()`/`expect()` di kode test dengan pesan deskriptif.
- Baca produksi utuh: `default_hotkeys` (statis), `CommandRegistry` (index HashMap + Vec, `register` replace-by-id benar), `apply_overrides` (input `hotkeys.json` → clone, tidak ada parse liar), `keystroke` (modifier/key tak dikenal → `None`), `plugin_of` (lookup CORE_PLUGINS + 7 prefix-unspelled exceptions, terdokumentasi).

## Temuan
- Trust boundary: input eksternal hanya `hotkeys.json` via `apply_overrides` — nilai hanya di-clone ke registry, tidak dieksekusi; id tidak dikenal didaftarkan unnamed (desain terdokumentasi).
- `keystroke` mengembalikan `None` untuk modifier/key yang tidak dikenal (baris 304, 327) — tidak ada panic path.
- `k[1..]` (baris 323) hanya dijalankan setelah `starts_with('F')` (ASCII 1-byte) — slicing aman.
- Duplikat/unknown id: test memastikan commands.txt unik + well-formed + names konsisten (test baris 343-368).

## Keputusan
Audit BERSIH — tidak ada item layak P0-P3. Tidak menambahkan perubahan (YAGNI). Tidak ada commit/push.

## Koordinasi
Klaim ledger baris terakhir + done. Hindari file session lain (workspace.rs/ops.rs/win_fs.rs/links.rs/reading/model.rs/config.rs/silica-templates/src/lib.rs) dan area upstream 0.5.11 (editor/preview/prompt/reading/highlight/Cargo.toml/README).
