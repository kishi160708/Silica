# UPGRADE-LOG — crates/silica-vault (zona sesi 5 arsitektur, branch burst/5-arsitektur)

Base ronde: 68884e7. Worktree: C:/Users/andre/code/Silica-ceo-ar5 (CARGO_TARGET_DIR C:/Users/andre/code/Silica-ar5-target).

[ronde 1] refactor | write_snapshot: `target.file_name().unwrap()` di jalur simpan snapshot (lib.rs:315 unix, lib.rs:320 fallback strip_prefix windows) — unwrap pada jalur produksi bisa panic kalau stem tidak menghasilkan file_name; nama sudah dibangun lokal di format! jadi ekstraksi dari path tak perlu | crates/silica-vault/src/lib.rs:292-322 | bukti: cargo fmt -p silica-vault --check exit 0; clippy -p silica-vault --all-targets -D warnings exit 0; test -p silica-vault exit 0 (46 passed); commit 0e75314 | [Hermes CEO ronde 1, sesi 5]

[ronde 2] P2 defer | `ends_with(".md")` case-sensitive di native_plugins.rs:107,284,495,549 (zona plugins, bukan vault) — file `Note.MD` dilewatkan diam-diam di filesystem case-insensitive; ubahan = perubahan perilaku lintas platform, ditunda menunggu keputusan owner | [Hermes CEO ronde 2, sesi 5]

[ronde 2] P3 defer | lib.rs:201 TODO(windows): std::fs::rename check-then-act pada rename note; sudah diketahui audit sebelumnya, butuh jaminan atomic replace Windows, bukan fix kecil | [Hermes CEO ronde 2, sesi 5]

[ronde 2] audit | ops.rs/registry.rs/win_fs.rs: unwrap/expect hanya di test module atau invariant "just inserted" (registry.rs:150); registry.rs save() sudah benar (temp+sync_all+rename+dir sync+cleanup on error); load() file korup = error InvalidData bukan overwrite senyap; tidak ada fix | [Hermes CEO ronde 2, sesi 5]
