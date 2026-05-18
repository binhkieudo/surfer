# Build Guide for Surfer

Hướng dẫn build Surfer từ source code.

## Yêu cầu

- **Rust**: phiên bản 1.92 trở lên
  - Cài đặt từ: https://rustup.rs/
- **Git**: để clone repository và submodules
- **GCC** (tuỳ chọn): nếu muốn enable `f128` feature (IEEE 754 quad precision)

## Chuẩn bị

### 1. Clone repository và khởi tạo submodules

```bash
git clone https://gitlab.com/surfer-project/surfer.git
cd surfer
git submodule update --init --recursive
```

Điều này sẽ tải xuống các thư viện con:
- `f128` - hỗ trợ số floating-point 128-bit
- `instruction-decoder` - bộ giải mã lệnh CPU

### 2. Kiểm tra Rust

```bash
rustc --version
cargo --version
```

Phiên bản tối thiểu: Rust 1.92

## Build Debug (Phát triển)

Build debug được tối ưu cho tốc độ biên dịch và gỡ lỗi, không tối ưu hiệu năng.

### Build toàn bộ workspace

```bash
cargo build
```

**Kết quả:**
- `target/debug/surfer` - trình xem waveform GUI (debug)
- `target/debug/surver` - server mode (debug)
- Thời gian: ~8-10 phút lần đầu

### Build chỉ một package

```bash
# Chỉ build surfer
cargo build -p surfer

# Chỉ build surver
cargo build -p surver
```

### Build với features tuỳ chọn

```bash
# Thêm hỗ trợ AccessKit (accessibility)
cargo build --features accesskit

# Thêm hỗ trợ f128 (quad precision)
cargo build --features f128

# Kết hợp nhiều features
cargo build --features "accesskit,f128"
```

### Chạy trực tiếp (Debug)

```bash
# Chạy surfer debug
cargo run -p surfer -- <file.vcd>

# Chạy surver debug
cargo run -p surver -- <file.vcd>
```

## Build Release (Sản xuất)

Build release được tối ưu cho hiệu năng (nhưng chậm hơn khi biên dịch).

### Build toàn bộ workspace

```bash
cargo build --release
```

**Kết quả:**
- `target/release/surfer` - trình xem waveform (optimized)
- `target/release/surver` - server mode (optimized)
- Kích thước: nhỏ hơn khoảng 10-15% so với debug
- Thời gian: ~15-20 phút lần đầu

### Build chỉ một package

```bash
cargo build --release -p surfer
cargo build --release -p surver
```

### Build với features (Release)

```bash
cargo build --release --features "accesskit"
cargo build --release --features "f128"
```

### Chạy trực tiếp (Release)

```bash
cargo run --release -p surfer -- <file.vcd>
cargo run --release -p surver -- <file.vcd>
```

## Install (Cài đặt toàn cục)

### Install từ local source

```bash
# Install debug version
cargo install --path surfer --debug

# Install release version (mặc định)
cargo install --path surfer
```

### Install trực tiếp từ GitLab

```bash
cargo install --git https://gitlab.com/surfer-project/surfer.git surfer
```

Các binary sẽ được cài vào `~/.cargo/bin/` (thường đã có trong PATH).

## Build WASM (Web)

Build cho web/browser:

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

Kết quả sẽ có ở `target/wasm32-unknown-unknown/release/`.

## Build mà không cần f128

Nếu gặp vấn đề với f128, có thể build mà không cần feature này:

```bash
cargo build --no-default-features
cargo build --release --no-default-features
```

**Lưu ý:** Điều này vô hiệu hóa `performance_plot` feature cùng.

## Cấu trúc Build

```
target/
├── debug/              # Debug artifacts
│   ├── surfer         # Executable (debug)
│   ├── surver         # Server executable (debug)
│   └── deps/          # Dependencies
└── release/           # Release artifacts
    ├── surfer         # Executable (release, optimized)
    ├── surver         # Server executable (release, optimized)
    └── deps/          # Dependencies
```

## Xóa Build Cache

```bash
# Xóa tất cả artifacts
cargo clean

# Rebuild từ đầu
cargo build --release
```

## Troubleshooting

### Lỗi: "no such file or directory (os error 2)" cho f128

**Giải pháp:** Khởi tạo submodules
```bash
git submodule update --init --recursive
```

### Build chậm

- Debug build được tối ưu cho biên dịch nhanh, dùng để phát triển
- Release build chậm nhưng kết quả nhanh hơn, dùng để production
- Lần đầu build sẽ lâu (dependencies)

### Cần GCC để build f128

```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# macOS
xcode-select --install

# Windows
# Dùng Visual Studio Build Tools hoặc MinGW
```

## Optimized Profiling Builds

Cho testing hiệu năng với debug symbols:

```bash
# Build với optimization level cao + debug info
cargo build -p surfer --config 'profile.dev.opt-level=3'
```

## Các môi trường được hỗ trợ

- **Linux**: x86_64, ARM64
- **macOS**: x86_64, ARM64 (Apple Silicon)
- **Windows**: x86_64 (MSVC, GNU)
- **WSL**: có hỗ trợ nhưng có một số issues với GUI framework

## Xem thêm

- [Tài liệu chính thức](https://docs.surfer-project.org/book/)
- [Repository GitLab](https://gitlab.com/surfer-project/surfer)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)
