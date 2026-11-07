# UPGRADE-LOG — crates/silica-plugins (zona sesi 5 arsitektur, branch burst/5-arsitektur)

Base ronde: 68884e7. Worktree: C:/Users/andre/code/Silica-ceo-ar5 (CARGO_TARGET_DIR C:/Users/andre/code/Silica-ar5-target).

[ronde 1] refactor | dedup pesan "'{id}' is not a command this plugin registered": 13 salinan identik (native.rs default trait, host.rs:609, native_plugins.rs x9 (DailyNotes/Backlink/OutgoingLink/Templates/RandomNote/WordCount/NoteComposer/ZkPrefixer/WebViewer/Slides/FileRecovery/AudioRecorder), vault_doctor.rs, live.rs:207 test) -> satu helper `native::unknown_command()`; sekaligus perbaiki native_plugins.rs:594 pesan menyesatkan: fallback hardcode 'publish:publish' untuk SEMUA id tak dikenal -> unknown_command(id) (param _id -> id) | crates/silica-plugins/src/{native.rs,host.rs,native_plugins.rs,vault_doctor.rs,live.rs} | bukti: cargo fmt -p silica-plugins --check exit 0; clippy -p silica-plugins --all-targets -D warnings exit 0 (setelah fix compile error E0425 id/putaran 1); test -p silica-plugins exit 0 (48 unit + 11 integrasi passed); commit ba3a20a | insiden: patch pertama memakai `id` padahal param `_id` -> clippy/test exit 101, ditemukan dari log gate, fixed sebelum commit | [Hermes CEO ronde 1, sesi 5]

[ronde 2] defer | case-sensitive `.md` di native_plugins.rs:107,284,495,549: file `Note.MD` dilewatkan plugin daily/random/publish di FS case-insensitive — perubahan perilaku, keputusan owner | [Hermes CEO ronde 2, sesi 5]

[ronde 2] audit | host.rs:416,504,505 Context::full(...).expect("...") — expect hanya untuk runtime baru yang baru saja sukses dibuat, tidak ada recovery berarti; tidak ditambah penanganan agar tidak menutupi invariant break | live.rs: thread_local LOADED/NATIVE terdokumentasi, reload key (id,enabled) sudah memperbaiki bug enable/disable; ensure() ganti host saat vault/settings berubah = tidak bocor antar-instance | native_plugins.rs vs CORE_PLUGINS (formats): 21 handler vs 30 id Obsidian — subset by design, retain_enabled pakai CORE_PLUGINS via commands crate, tidak duplikasi config | [Hermes CEO ronde 2, sesi 5]

[ronde 3] lihat catatan ronde 3 di bawah bila ada.
