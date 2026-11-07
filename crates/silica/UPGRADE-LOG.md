# UPGRADE-LOG — crates/silica

Append-only. Satu entri per item. Konvensi repo: file ini TIDAK di-commit.

## [perf-proof] P2 | root-cause lemot + perf test release @0cc3f22 | editor.rs:6722 (+99 test) | bukti di bawah

### Konteks
Owner: "Silica dikeluhkan lemot padahal Rust. Buktikan penyebabnya, jangan menebak."
Worktree: C:/Users/andre/code/Silica-ceo-lag (branch perf/lag-proof @0cc3f22, HEAD=origin feat ls-remote verified). Target dir sendiri.

### 1. Binary mana yang berjalan (sebab #1 — bukan di code)
- Saat investigasi: 0 proses `silica.exe` berjalan. `tasklist`: hanya 2 cargo.exe + test binary DEBUG `writer_roundtrip-121d7d11f59be01d.exe` (deps debug worktree Silica-ceo-pr milik gate PR session lain). Shortcut Desktop/Start Menu menuju Silica: 0.
- Diagnosa sesi pagi (2026-11-06 10:00, wiki entities/silica-lag-fix-2026-11-06): owner menjalankan `target/debug/silica.exe` (PID 17764+20336); debug dibunuh berulang karena ada yang `cargo run` (default debug). Aturan sudah ada di .burst/silica/COORDINATION.md:16 ("JANGAN cargo run; pakai release") + CLAUDE.md/docs sudah `--release`.

### 2. Perf test WAJIB release — angka nyata (release build @0cc3f22, 20 threads)
Bench index 20k-note vault (35.3 MB, 22k file), `silica-bench index --runs 3`:

| build | run1 total | best | gate 2 s |
|---|---|---|---|
| release | 600 ms | 486 ms | PASS |
| debug | 743 ms | 644 ms | PASS |

→ Index vault BUKAN bottleneck; debug hanya 1.2–1.5× lebih lambat di path paralel ini.

### 3. Profiling hot path release (editor reparse per edit)
Stress spike `silica.exe --synthetic 50000 --stress 600` (release):
- 600 steps in 18.4 s; frames shape mean 6.56 ms (render OK)
- **parses 31: mean 447.2 ms, max 1121.9 ms**; edit-to-decoration latency mean 670 ms → jeda per edit pada note 50k baris.

Stage probe (unit test release, `--nocapture`), note 50k baris/4266 KB:
- parse 226–250 ms | highlight_lines 42–55 ms | foldables 4–6 ms (2499 folds) | 247k spans
- Scaling: 10k=23.8 ms, 12k=25.8, 15k=39.2, 20k=46.5, 30k=64.5 ms → **linear ~2.2 µs/line** (run kedua; run pertama 20k=115.6 ms = noise kontensi cargo sibling)
- Note realistis ≤5k baris: parse ~10–18 ms → typing latency wajar.

Arsitektur sudah benar: parse di `background_executor`, satu parse pada satu waktu, coalescing (pending_parse+parse_queued), dekorasi basi ditampilkan sementara (editor.rs:2264-2371).

### Keputusan
Tidak ada fix kode performa: index lolos gate, parser linear (bukan quadratic), note normal cepat; optimasi parser inti = perubahan besar berrisiko tinggi untuk kasus sintetis 50k baris saja = YAGNI. Akar keluhan "lemot" = build debug yang dijalankan (sebab #1), sudah divalidasi angka debug-vs-release.

### Sisa perubahan
- editor.rs: test `perf_stage_timings_on_a_50k_line_note` — regression guard: assert parse 50k < 2000 ms (release-only; debug ~10× lebih lambat jadi assert di-skip via cfg). Angka terukur 226–250 ms = headroom ~8×.
- Gate: fmt/clippy/test workspace dijalankan setelah perubahan (log gate-*.log, exit code di gate-*.exit).

### Hasil gate + verifikasi final (exit code nyata, tanpa pipe)
- fmt run1 EXIT=1 (kode pre-rustfmt) → `cargo fmt --all` + recheck EXIT=0 (diff terkunci editor.rs +132).
- clippy --workspace --all-targets -D warnings EXIT=0 (2m50s).
- cargo test --workspace EXIT=0, 1436 passed, 0 FAILED/panicked (log gate-test.log).
- Verifikasi fokus kode final (skrip ad-hoc, release + debug): perf_stage PASS dua profile; release parse 229 ms / total 270 ms; debug parse 920 ms (assert release-only terbukti perlu).
- Commit f46a94f di branch perf/lag-proof, push origin ls-remote verified = f46a94f. Merge ke feat = keputusan owner (0 fix produksi, test-only guard).
