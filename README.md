# ClipTray

Riwayat clipboard ringan untuk Windows yang tinggal di system tray. ClipTray mencatat setiap teks yang kamu copy, lalu menampilkannya di popup kecil supaya bisa disalin ulang kapan saja. Cocok sebagai pengganti `Win+V` saat fitur bawaan Windows itu tidak berfungsi atau terlalu terbatas.

Dibangun dengan **Tauri 2**, **Rust**, **Svelte 5**, dan **SQLite**. Installer berukuran sekitar 2 MB.

## Fitur

- **Pencatatan otomatis.** Setiap teks yang di-copy dari aplikasi mana pun langsung tercatat. Pencatatan berbasis event (`WM_CLIPBOARDUPDATE`), bukan polling.
- **Popup cepat.** Buka dengan `Ctrl+Shift+V` atau klik ikon tray. Popup muncul di dekat kursor dan bisa digeser lewat header.
- **Klik untuk salin ulang.** Item langsung masuk ke clipboard, lalu tinggal `Ctrl+V` di tujuan.
- **Pencarian** di seluruh riwayat.
- **Deteksi jenis isi.** Item dikelompokkan otomatis menjadi Teks, Link, Kode, atau Warna, dengan filter untuk masing-masing. Kode ditampilkan dengan font monospace, warna ditampilkan sebagai contoh warna.
- **Sematkan (pin).** Item penting selalu tampil di atas dan tidak ikut terhapus saat riwayat dibersihkan.
- **Tanpa duplikat.** Teks yang sama tidak dicatat dua kali; item lama cukup naik ke paling atas.
- **Jeda pencatatan** dari popup atau menu tray.
- **Aman untuk password manager.** Isi yang ditandai `ExcludeClipboardContentFromMonitorProcessing` (KeePass, 1Password, Bitwarden, dan lainnya) tidak dicatat.
- **Tema Terang, Gelap, atau mengikuti Windows.**
- **Start with Windows** (opsional).
- **Tersimpan lokal.** Riwayat bertahan setelah restart, maksimal 500 item terbaru ditambah item yang disematkan. Tidak ada data yang dikirim ke internet.

## Pintasan keyboard

| Tombol | Fungsi |
|---|---|
| `Ctrl+Shift+V` | Buka / tutup popup dari aplikasi mana saja |
| `↑` `↓` | Pilih item |
| `Enter` | Salin item terpilih |
| `Ctrl+1` – `Ctrl+9` | Salin cepat item ke-1 sampai ke-9 |
| `Ctrl+P` | Sematkan / lepas item |
| `Ctrl+D` | Hapus item |
| `Tab` / `Shift+Tab` | Ganti filter |
| `Esc` | Kosongkan pencarian, lalu tutup popup |

Menu klik kanan di ikon tray berisi: Buka, Jeda pencatatan, Hapus semua, Start with Windows, dan Keluar.

## Instalasi

Unduh installer dari halaman **Releases**, lalu pilih salah satu:

- `ClipTray_x.y.z_x64-setup.exe` (NSIS, disarankan)
- `ClipTray_x.y.z_x64_en-US.msi`

Setelah dijalankan, ClipTray tidak membuka jendela apa pun. Cari ikonnya di system tray (klik panah `^` di taskbar jika tersembunyi).

Persyaratan: Windows 10 atau 11 dengan WebView2. WebView2 sudah terpasang di Windows 11 dan Windows 10 yang diperbarui; jika belum ada, installer akan mengunduhnya.

## Build dari source

### Prasyarat

- [Rust](https://rustup.rs) dengan toolchain MSVC
- Visual Studio Build Tools dengan workload **Desktop development with C++**
- Node.js 18 atau lebih baru

### Menjalankan

```bash
npm install
npm run tauri dev     # mode development
npm run tauri build   # buat installer .exe dan .msi
npm run release       # build, lalu salin hasilnya ke folder release/
```

`npm run release` mengumpulkan `ClipTray.exe` (portable) serta installer `.exe` dan `.msi` ke folder `release/`. Hasil mentahnya ada di `src-tauri/target/release/bundle/`. Kompilasi Rust pertama kali bisa memakan waktu cukup lama; build berikutnya jauh lebih cepat.

### Perintah lain

```bash
npm run check                          # type-check Svelte + TypeScript
cd src-tauri && cargo test             # unit test Rust
npx tauri icon app-icon.png            # buat ulang ikon dari app-icon.png
```

## Struktur proyek

```
cliptray/
├── src/                     # Frontend (Svelte 5 + TypeScript)
│   ├── App.svelte           # Popup: daftar, pencarian, filter, pengaturan
│   └── lib/
│       ├── api.ts           # Wrapper invoke() dan event ke backend
│       ├── Icon.svelte      # Komponen ikon SVG
│       └── icons.ts         # Kumpulan path ikon
└── src-tauri/               # Backend (Rust)
    ├── tauri.conf.json      # Konfigurasi window popup dan bundler
    ├── capabilities/        # Izin Tauri 2
    └── src/
        ├── lib.rs           # Setup: plugin, tray, hotkey, posisi popup
        ├── watcher.rs       # Listener clipboard (thread terpisah)
        ├── db.rs            # SQLite: simpan, cari, pin, hapus, deteksi jenis
        └── commands.rs      # Command yang dipanggil dari frontend
```

## Data

Riwayat disimpan di `%APPDATA%\com.cliptray.app\history.db`. Hapus file tersebut untuk mengosongkan riwayat sepenuhnya.

Batasan saat ini: hanya teks yang dicatat (gambar dan file belum), dan teks di atas 1 MB diabaikan.
