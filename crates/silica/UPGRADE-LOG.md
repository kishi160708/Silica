
[ronde 10] P1 | site export: replace_attr needle "href=\"" (tanpa spasi) cocok di dalam data-href=\" -> data-href ikut di-rewrite jadi URL, href asli (teks link) dibiarkan -> semua link internal hasil export mati di browser; bukti runtime smoke: _site/Sub/Deep/Plan.html data-href=\"../../README.html\" href=\"README.md#top\" pra-fix | crates/silica/src/site.rs:359 (fix: needle space-prefixed, konsisten dgn remove_attr) + test regresi a_link_rewrite_replaces_href_and_never_its_data_href | bukti: smoke harness scratch/site-smoke EXIT=101 pra-fix -> EXIT=0 pasca-fix (data-href utuh, href=../../README.html, embed src walk benar); gate merge 5b4a66b: fmt=0, cargo test -p silica site:: 8 passed/0 failed exit 0; commit 55b5ab1, push verified ls-remote=5b4a66b | catatan: unit test upstream lolos selama ini karena assert contains("href=\"Target.html\"") juga match di dalam data-href (false positive sama) | [Hermes CEO ronde10]

## ronde 12 — audit berlapis crates/silica/src/graph/ (Hermes CEO, worktree Silica-ceo-r13, base f42fbf9)

`[ronde 12] AUDIT BERSIH, TIDAK ADA FIX | zona: crates/silica/src/graph/{mod.rs,layout.rs,view.rs,image.rs} | diff sejak audit terakhir (ronde1 base 6469944) hanya view.rs +49/-9 = bee31ff perf shaped-label cache + cba98e6 fmt + merge 3c68f36; bukti gate tingkat crate di worktree r13 (target dir sendiri, tanpa pipe): cargo fmt --all -- --check EXIT=0; cargo clippy -p silica --all-targets -- -D warnings EXIT=0; cargo test -p silica EXIT=0 (560 passed, 0 failed, 3 test-binary) | TIDAK ada fix/commit/push`

### Audit kode baru bee31ff (label cache)

- **Invalidasi**: `labels` di-reset di `rebuild()` (view.rs:665) — SATU jalur rebuild tunggal (dipanggil dari 397/480/491/521/963/1148). Node set berubah → cache dibuang. Benar.
- **Kelengkapan key**: `(label, [color bits x4], font_family)` — `font_size` konstan `px(11.)` (view.rs:2339, satu-satunya occurrence), family di-capture per paint dari `window.text_style()`, warna per-node via `run_color`. Tidak ada input shaping lain yang bervariasi → tak ada salah-hit cache.
- **Lock**: poison ditangani sadar (`unwrap_or_else(|e| e.into_inner())`), `drop(cache)` sebelum `line.paint(...)` — lock tidak dipegang selama rasterisasi.
- **Guard lama utuh**: `labels < 400` + threshold dim/scale tetap diterapkan sebelum populate cache.
- **layout.rs insert() unwrap (baris 237/244, kode PRODUKSI — test mod mulai 320)**: invarian aman — di 244 `children` selalu `Some` (entah sudah tersubdivide, entah baru dibuat `subdivide` di arm 233-240). Pola sengaja (barnes-hut), bukan panic-path.
- **image.rs / mod.rs**: 0 unwrap/expect/panic/unsafe produksi.

### Observasi TANPA fix (P3, diukur, tidak memenuhi ambang fix)

- P3: entri cache yang warna/themanya sudah tak dipakai (ganti tema/config warna tanpa rebuild) tetap hidup sampai rebuild/node-set berikutnya — pertumbuhan terbatas dan trade-off didokumentasikan upstream ("a shaped line outlives any single paint"). Fix eviction = sentuh kode upstream segar tanpa masalah terbukti = YAGNI + risiko divergensi fork.
- P3 (catatan perf): key tetap mengalokasikan `String` per label per paint; hanya shaping yang di-cache. Alternatif key `SharedString` = penghematan refcount-level, tidak sebanding dengan risikonya.

## ronde 13 — audit + fix strip_active (site export) (Hermes CEO, worktree Silica-ceo-r13, base f42fbf9)

`[ronde 13] P1 FIXED | crates/silica/src/site.rs:strip_active | celah: (1) pencarian case-sensitive -> <SCRIPT> lolos utuh; (2) needle close dibangun dari tag yang masih membawa '<' (`</<script`) -> TIDAK PERNAH match, jadi hanya tag pembuka yang dibuang dan payload tetap ada di halaman terbit sebagai teks yang bisa di-patch; (3) sekali-pass menyisakan potongan yang menyambung (<scr<script>x</script>ipt> -> <script> hidup) | bukti merah: 3 test baru FAILED exit 101 pada kode lama (test sibling lolos karena assertion hanya cek "<script" bukan payload); bukti hijau pasca-fix: cargo test -p silica site:: 14 passed exit 0; gate: cargo fmt --all -- --check EXIT=0; cargo clippy -p silica --all-targets -- -D warnings EXIT=0 | commit 5a20e38 branch upgrade/site-strip-active, push origin ls-remote = 5a20e38 = HEAD verified; merge-tree feat vs branch = clean exit 0 | base f42fbf9 (saat kerja feat milik sibling maju ke 68884e7 = merge upstream 0.5.16; rebase/merge ke feat = keputusan owner)`

### Detail

- **Konteks trust boundary**: `silica-markdown` emit `BlockKind::HtmlBlock => push_str(content)` verbatim — HTML mentah sebuah note sampai mentah ke export. `strip_active` adalah satu-satunya penyaring.
- **Fix**: (a) pencarian case-insensitive lewat `to_ascii_lowercase` (hanya A-Z, panjang byte identik → index aman untuk slice string asli); (b) close needle `format!("</{}", &tag[1..])`; (c) pass diulang sampai output stabil — satu pass menyisakan `<scr` + `ipt>` yang menyambung jadi `<script>`; terminasi terjamin: tiap pass produktif memperpendek output.
- **Test baru**: `an_uppercased_script_tag_is_dropped_too`, `a_split_script_tag_cannot_reassemble_into_a_live_one`, `a_script_elements_content_goes_with_it` (assert cek payload `alert(1)`, bukan hanya `<script`).
- **Observasi TANPA fix (P2, di luar zona)**: denylist hanya `<script`/`<iframe` — tag ber-event-handler lain (`<img onerror>`, `<svg onload>`, `<object>`) dan `javascript:` URL dari raw HTML block tidak disentuh; memperbaikinya = saringan allowlist tag/atribut = perubahan besar di renderer/eksport, butuh keputusan owner + desain.

## ronde 14 — audit kode baru upstream 0.5.16 (Hermes CEO, worktree Silica-ceo-r13, base 68884e7)

`[ronde 14] AUDIT BERSIH, TIDAK ADA FIX | zona: diff 47be51d perf(workspace) reading model per-mode — crates/silica/src/workspace.rs (+95/-6: reading_stale, build_reading_model, sync_reading, toggle_tag_folds/tag_fold_next, bookmark view-header action) + crates/silica/src/reading/view.rs (+5: blocks count di state JSON) + crates/silica/tests/e2e.rs (+34 bookmark) | bukti gate tingkat crate di worktree r13 (target dir sendiri, tanpa pipe): cargo fmt --all -- --check EXIT=0; cargo test -p silica EXIT=0 (562 passed, 0 failed); cargo clippy -p silica --all-targets -- -D warnings EXIT=0 | TIDAK ada fix/commit/push`

### Temuan (semua CLEAN)

- **Invariant reading_stale**: dibaca hanya di `sync_reading` (guard: mode editing/Graph/non-markdown/stale-false → return) dan ditulis false hanya di `build_reading_model`. Kombinasi mode+flag mencakup semua rute: `set_mode` (:5675), `load_tab` (:9848), `open_path` (:5672) — ketiganya memanggil sync_reading; buka note langsung di mode Reading → `build_reading_model` langsung. Buffer kotor menang atas disk (autosave 2s tertinggal) — urutan benar; flush_save di set_mode sudah menangani kasus editing→reading.
- **tag_fold_next**: state machine 3-klik cocok dengan doc/behavior Obsidian (klik-1 clear = no-op, klik-2 fold semua, klik-3 buka); `toggle_tag_hierarchy` me-reset ke false. `toggle_tag_fold` per-key tidak mengubah flag — aman.
- **reading/view.rs**: `blocks` count di state JSON = data read-only, tak ada efek render.
- **Bookmark action view-header**: `when(view.current.is_some())` + reuse prompt `start_bookmark` — satu jalur dengan perintah palette; tak ada validasi yang hilang.
- **Perf**: model reading hanya dibuild untuk mode yang menampilkannya; mode editing membayar `build_with("","")` kosong + set_note (murah) — trade-off terdokumentasi, acceptance test e2e (+34) hijau.
- Observasi P3 (deliberate upstream): pesan status build-time kini hanya di `SILICA_DEBUG_STATUS` — kemampuan debug tetap ada, hanya tak lagi menempel di status bar biasa.

## ronde 15 — audit kualitas assertion test repo-wide (Hermes CEO, worktree Silica-ceo-r13, base 68884e7)

`[ronde 15] P2 FIXED (test-only) | 3 assertion yang tidak bisa gagal | (1) silica-highlight src/lib.rs:147 every_grammar_compiles membuang hasil compile — compile_pattern sengaja `Regex::new(...).ok()` (gagal = None senyap) sehingga grammar rusak lolos sebagai "match nothing"; bukti: mutasi satu regex grammar → FAILED exit 101 'arduino: the macro pattern did not compile', test lama akan lolos; (2) silica-pdf tests/document.rs render_timings hanya println — kini assert setiap halaman fixture ekstrak teks; (3) silica-cache tests/rename_corpus.rs attachments_rename_too mem-filter corpus markdown-only, dapat nol, lalu return — jalur rename attachment TIDAK PERNAH diuji; diganti vault sintetis (note embed+link attachment, attachment di-rename, link harus ikut) lewat check_vault bersama | bukti: cargo test -p silica-highlight/-p silica-pdf/-p silica-cache semua EXIT=0; fmt --all --check EXIT=0; clippy 3 crate --all-targets -D warnings EXIT=0 | commit cdcc1e5 branch upgrade/test-assertions, push origin ls-remote = cdcc1e5 = HEAD verified | scan: 1427 test, 59 zero-assertion — 56 false-positive (helper shows/refuses/check ber-assert), 3 nyata (di atas), 0 lolos seleksi`

- **Metode**: brace-counting scan 1427 `#[test]` → kandidat zero-assertion diverifikasi manual (helper function test memuat assert = bukan temuan).
- **Pelajaran (generalisasi ronde13)**: assertion lemah = bug lolos bertahun-tahun. Test dengan hasil dibuang (`let _ =`), early-return, atau hanya `println` = coverage palsu.
- **Observasi TANPA fix**: `document_is_send_and_sync` (pdf) = assertion tingkat tipe, benar by design; `every_grammar_compiles` kini walk penuh — biaya runtime naik sedikit (compile sekali+cache), tanpa cache poisoned handling berubah.
- **Catatan verifikasi**: `render_timings` adalah `#[ignore]` by-design (probe timing manual, 0.5s) — assertion-nya diverifikasi via `cargo test -p silica-pdf -- --ignored render_timings` (1 passed, EXIT=0), bukan lewat suite normal. Skrip ad-hoc `hermes-verify-` yang memfilter per-nama juga membuktikan metacel: filter zero-match tetap EXIT=0 — verifikasi wajib cek "1 passed", bukan hanya exit code.
