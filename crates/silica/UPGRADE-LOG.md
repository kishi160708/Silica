# UPGRADE-LOG — crates/silica (site.rs, upstream 0.5.14 export feature)

Format: `[ronde N] prioritas | item | file:baris | bukti`

- [ronde 8] P1 | export: link ke note bernama karakter entity (`A & B.md`) gagal resolve — renderer escapes href jadi `A &amp; B`, resolve() mencari mentah di map → href dihapus, link halaman mati | crates/silica/src/site.rs:297 (rewrite), unescape_attr :203 | fix: unescape_attr sebelum resolve; test a_link_named_with_an_ampersand_resolves_through_the_escaped_attr
- [ronde 8] P2 | export: title halaman tidak di-escape (`<title>{title}</title>`) — `&` legal di nama file semua platform → markup tak valid | crates/silica/src/site.rs:105 | fix: escape_title (&,<,>); test the_page_title_escapes_the_three_hostile_characters
- [ronde 8] P3 | export: copy_assets/rel_url tidak URL-encode spasi/`&` dalam href — browser toleran; deferred
- [ronde 8] audit | slicing rewrite() seluruhnya berbasis needle ASCII (find/rfind) → UTF-8-safe; notes[0] unwrap dijaga cek notes.is_empty() (site.rs:42); SKIPPED_DIRS menutup .obsidian/.trash/.git; read_verified memakai sane_relative upstream (reject ../ dan absolute) — tanpa fix tambahan
