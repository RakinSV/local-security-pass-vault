#!/usr/bin/env bash
# scripts/dev-setup.sh — первоначальная настройка окружения разработки

set -euo pipefail

echo "=== VaultPass Dev Setup ==="

# Проверка Rust
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo "Rust: $(rustc --version)"

# Инструменты безопасности
echo "Installing security tools..."
cargo install cargo-audit --quiet
cargo install cargo-vet --quiet

# Nightly для fuzzing
rustup toolchain install nightly --quiet
cargo +nightly install cargo-fuzz --quiet

# Системные зависимости
if command -v apt &> /dev/null; then
    # Ubuntu/Debian
    sudo apt-get install -y \
        libsodium-dev \
        libsqlite3-dev \
        libssl-dev \
        pkg-config \
        libsecret-1-dev \  # libsecret для Linux keychain
        libwebkit2gtk-4.1-dev  # для Tauri
elif command -v dnf &> /dev/null; then
    # Fedora
    sudo dnf install -y libsodium-devel sqlite-devel openssl-devel libsecret-devel webkit2gtk4.1-devel
fi

# Node.js для расширения и Tauri UI
if ! command -v node &> /dev/null; then
    echo "Install Node.js 20+ manually: https://nodejs.org"
    exit 1
fi

echo "Node: $(node --version)"

# Первоначальная проверка зависимостей
echo ""
echo "=== Security Audit ==="
cargo audit

echo ""
echo "=== Setup Complete ==="
echo "Next steps:"
echo "  cd core-vault && cargo test    # запустить тесты"
echo "  cargo audit                    # проверить CVE"
echo "  cargo fuzz list                # список фаззеров"
