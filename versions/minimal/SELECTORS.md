# Selector Reference (`versions/static`)

Dokumen ini menjelaskan seluruh selector yang dipakai oleh versi static, khususnya untuk parsing halaman Komiku.

## 1) Selector Elemen UI Lokal (halaman app)

Selector berikut dipakai untuk mengikat elemen DOM di app:

- `#inputPanel`
- `#readerPanel`
- `#openForm`
- `#targetUrl`
- `#chapterTitle`
- `#chapterMeta`
- `#pages`
- `#nextBtn`

Catatan:
- Ini adalah selector untuk elemen di `index.html` milik app, bukan selector dari website Komiku.

## 2) Selector Judul Chapter (dari HTML Komiku)

Dipakai di fungsi `extractTitle` dengan urutan prioritas:

1. `div header h1`
2. `h1` (fallback)

Perilaku:
- App mengambil `textContent.trim()` dari elemen pertama yang valid.
- Jika semua gagal, judul fallback: `Manga Chapter`.

## 3) Selector Gambar Chapter (dari HTML Komiku)

Dipakai di fungsi `extractImageUrls` dengan urutan prioritas:

1. `#Baca_Komik img`
2. `div#Baca_Komik img`
3. `img` (fallback global)

Urutan atribut URL gambar yang dibaca:

1. `src`
2. `data-src`
3. `data-lazy-src`

Filter tambahan:
- Abaikan URL kosong.
- Abaikan data URI (`data:`).
- URL direlative-kan ke `sourceUrl` menggunakan `new URL(raw, sourceUrl)`.
- URL duplikat dihapus (dedupe memakai `Set`).

Perilaku penting:
- Jika selector prioritas lebih tinggi sudah menghasilkan daftar gambar (`output.length > 0`), selector berikutnya tidak dieksekusi.
- Jika hasil akhir `0` gambar, chapter dianggap gagal diparse.

## 4) Selector Next Chapter (dari HTML Komiku)

Dipakai di fungsi `extractNextUrl` dengan urutan prioritas:

1. `a[rel='next']`
2. `a.next`
3. `.next a`
4. `.navig a`
5. `.pagination a`
6. `a` (fallback global)

Aturan validasi link:
- `href` tidak boleh kosong.
- `href` bukan `#`.
- `href` tidak boleh diawali `javascript:`.
- URL absolut harus valid dan tidak sama dengan `sourceUrl`.

Heuristik pemilihan:
- Prioritas langsung jika:
  - atribut `rel` mengandung `next`, atau
  - teks link mengandung `next`, atau
  - teks link mengandung `selanjutnya`.
- Jika tidak ketemu:
  - app mencoba baca nomor chapter saat ini dari pathname (`chapter-<angka>`),
  - lalu cari kandidat yang mengandung `chapter-(angka+1)`.
- Fallback terakhir:
  - dari semua kandidat yang mengandung `chapter-`, pilih kandidat terakhir.

## 5) Catatan Kompatibilitas

- Selector di atas disusun untuk struktur Komiku saat ini, namun bisa berubah jika HTML Komiku berubah.
- Jika parsing mulai sering gagal, titik pertama yang dicek adalah:
  - selector gambar (`#Baca_Komik img`),
  - selector next chapter (`a[rel='next']`, `.next a`, `.pagination a`).
