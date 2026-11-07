# UPGRADE-LOG — crates/silica-mcp (base fd11db1)

Baseline pre-fix: re-run flake pada workspace paralel = cargo test --workspace EXIT=101, 1 fail: app.rs:817 WSAETIMEDOUT (log gate-0cc3f22.log); crate sendirian EXIT=0 (27 pass) -> indikasi kontensi.

[ronde 2026-11-06 ~21:40] P2 (test-only) | a_timed_out_call_costs_only_its_own_connection flaky saat workspace paralel | crates/silica-mcp/src/app.rs:817 | akar: Slow::app config timeout 60ms (sengaja, supaya 'hang' timeout saat reply masih in flight) juga membatasi call ping kedua; fix: call_within(..., 10s) utk call kedua; bukti pasca-fix: cargo test --workspace EXIT=0 (1436 passed), targeted x10 10/10, ad-hoc mechanism check EXIT=0 (temp, cleaned). Produksi tidak disentuh.

deferred: P2 — kalau flake lain semacam ini muncul, kandidat: test-threads=1 untuk test socket atau marker serial (butuh dep baru) — tambah hanya kalau ada kasus kedua.
