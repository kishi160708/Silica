# UPGRADE-LOG — silica-math

Format: `[ronde 4] prioritas | item | file:baris | bukti (perintah + exit code)`

- [ronde 4] baseline | gate | crates/silica-math | `cargo test -p silica-math` exit 0 (20 passed) ; `cargo clippy -p silica-math --all-targets -- -D warnings` exit 0 ; `cargo fmt -p silica-math -- --check` exit 0
- [ronde 4] P1 | rekursi `collect` tanpa batas kedalaman | crates/silica-math/src/mathjax.rs:187 (pra-fix) | `collect` memproses group yang memuat deklarasi (`\newcommand`/`\def`/`\require`) dengan rekursi `collect(body)` tanpa depth guard — dua fungsi `expand` memakai `DEPTH_CAP`=16 (mathjax.rs:275, :380) tapi `collect` tidak. Note berisi `\frac{{\frac{{\frac{...}}}}` bersarang ribuan level → stack overflow (SIGSEGV, tidak catchable) di jalur render. Reproduksi: nested `\frac` depth ≈ ribuan level dari input ~100KB.
- [ronde 4] P1 fix | cap rekursi | mathjax.rs:140-151 (wrapper `collect` + `collect_at`), :191-201 (guard `depth < DEPTH_CAP`; pada/melewati cap group dikeluarkan apa adanya sehingga teks tetap sampai ke parser) | diff terkecil: 1 param depth + 1 percabangan; perilaku nest dangkal tidak berubah
- [ronde 4] regression test | `collect_stops_recursing_past_the_depth_cap` | mathjax.rs:592-607 | nest dangkal → deklarasi tetap terkumpul (output tanpa `\newcommand`); nest > cap → teks bertahan (bukti walk berhenti)
- [ronde 4] verifikasi ulang | gate | crates/silica-math | `cargo test -p silica-math` exit 0 (21 passed: 20 lama + 1 regression) ; clippy exit 0 ; fmt exit 0

Status: fix dikerjakan (P1), gate hijau. Commit di branch `upgrade/math`, push ke origin fork (fetch→rebase→push, tanpa force; ls-remote diverifikasi).
