# UPGRADE-LOG — silica-text

## [ronde8b, 2026-11-06] verifikasi berjalan — log lama tetap valid, audit ulang DICABUT
- Klaim ronde8b (re-claim silica-text di base c55a6b5) dicabut sebagai duplikat.
- Bukti: `git diff 6469944..c55a6b5 -- crates/silica-text` = kosong (upstream 0.5.12/0.5.13 tidak menyentuh crate ini). UPGRADE-LOG audit pada base 6469944 di worktree Silica-upgrade-loop (`crates/silica-text/UPGRADE-LOG.md`) tetap valid — tidak ada perubahan kode sejak audit itu.
- Audit ulang pada kode identik = pemborosan ronde (protocol upgrade-loop §7). Tidak ada edit, tidak ada commit.
- Verifikasi koordinasi: worktree Silica-ceo-text (base c55a6b5, branch upgrade/text-r9) status bersih; main tree tidak disentuh.
- Ronde berikutnya yang layak untuk crate ini: hanya setelah ada commit baru yang menyentuh `crates/silica-text/`.

---

# Audit r8 — worker READ-ONLY Silica-ceo-text (branch `upgrade/text-r8` @ c55a6b5)

Tanggal: 2026-11-06. Base pembanding sesuai brief: `4ab8ec7`.
Lingkungan: PATH MSVC/rustup diekspor sebelum setiap perintah cargo.

## Gate

| Gate | Exit | Hasil |
|---|---|---|
| `cargo test -p silica-text` | 0 | 115 passed / 0 failed (100 unit + 15 integration `tests/writer_roundtrip.rs` + 0 doc) |
| `cargo clippy -p silica-text --all-targets -- -D warnings` | 0 | bebas warning |
| `cargo fmt -p silica-text -- --check` | 0 | bebas diff |

## Diff

`git diff 4ab8ec7 c55a6b5 -- crates/silica-text` → kosong (exit 0). Rentang itu hanya
menyentuh crate `math` (`5b9a2cf fix(math): cap collect recursion at DEPTH_CAP`).

## Temuan (grep produksi + baca manual buffer/find/wrap/autopair/format/table/lists/plan/spell)

- `TODO`/`FIXME`/`XXX`, `unsafe`, `transmute`, `panic!`, rekursi: nihil di seluruh kode
  produksi. Tidak ada alokasi O(n^2) per karakter.
- `unwrap()`/`expect()` produksi hanya 4 tempat, semuanya tak tercapai:
  `format.rs:204,210` (dijaga `list_state`), `format.rs:652` (dijaga flag `all`),
  `table.rs:402` (`peek()` menjamin Some). Sisa `unwrap` ada di `#[cfg(test)]`
  (`find.rs:178,186`, `autopair.rs:213`).
- Nihil P0/P1/P2. Lima P3 (kontrak API, bukan bug terekspos):
  1. `autopair.rs:123,153,158,174,182` + `buffer.rs:367,376,641,893,1376,1415` — slicing
     `text[..]` tanpa guard di API publik: panic jika offset bukan char boundary / di
     luar rentang. Pemanggil internal selalu terjaga; prasyaratnya tidak tertulis.
  2. `spell.rs:41-63` — `misspelled` hanya memindai run ASCII; kata non-ASCII tak pernah
     diperiksa ejaan.
  3. `format.rs:130` — `for pre in [0, 4]` meng-hardcode emoji warna ≤ 4 byte.
  4. `buffer.rs:133-156` — `FileText::parse` tak kenal ending `\r` tunggal; byte tetap
     utuh saat save (bukan kehilangan data).
  5. Tradeoff performa terbatas & terdokumentasi: `rebuild_lines` O(n)/edit,
     `table::apply` render ulang tabel, `MAX_MATCHES` 20.000, `next_footnote_number`
     compile regex per panggilan.

## Edge case (baca manual + test nyata, lulus)

- Empty buffer: `act("",0,'*')` → `Close('*')`; `compile("")` → `None`;
  `misspelled("")` kosong; `row_of` slice kosong = 0.
- Multi-byte UTF-8: langkah/hapus per grapheme cluster (`floor_char_boundary`,
  `GraphemeCursor`) — lulus `cafe\u{301}`, flag, keluarga ZWJ, `(\u{301})`; `find` pada
  byte range regex, query ASCII atas teks non-ASCII.
- Off-by-one: byte di wrap point = awal row berikutnya (`row_of`); `next_from`/
  `previous_from` dibungkus dengan guard `is_empty`; `cursor_at`, `swap_lines_plan`
  menjaga indeks 0 dan `len-1`.

## Verdict

BERSIH P0-P2. Diff vs `4ab8ec7` kosong, ketiga gate hijau, temuan hanya pengerasan
kontrak P3. Tidak ada file kode diubah; audit READ-ONLY.
