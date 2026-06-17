# VaultPass — локальный менеджер паролей

> Полная архитектура и контекст разработки для Claude Code.
> Правила в `.claude/rules/` — читай их при работе с конкретными компонентами.

## Что это

Локальный password manager без облака, без телеметрии, без сетевых запросов.
Клиенты v1: Windows + Linux (Tauri 2) + браузерные расширения Chrome и Firefox.

## Быстрый старт

```bash
cargo build                    # собрать всё
cargo test                     # все тесты
cargo audit                    # проверка CVE в зависимостях
cargo fuzz run vault_open      # фаззинг (требует nightly)
```

## Структура проекта

```
vaultpass/
  core-vault/        # Rust crate — крипто-ядро, модель данных, БД
  desktop/           # Tauri 2 приложение (Win + Linux)
  extension/         # Браузерное расширение (Chrome + Firefox, MV3)
  docs/
    adr/             # Architecture Decision Records
    threat-model.md  # Полная модель угроз
  scripts/           # build, sign, release скрипты
  CLAUDE.md          # этот файл
  .claude/rules/     # детальные правила по компонентам
```

## Стек

| Компонент | Технология | Почему |
|-----------|-----------|--------|
| Крипто-ядро | Rust + libsodium (sodiumoxide) | Аудированная библиотека, всё в одном месте |
| Desktop | Tauri 2 | Rust-процесс для крипто, WebView только для UI, ~8 МБ бинарь |
| Расширение | TypeScript + Manifest V3 | Стандарт Chrome/Firefox |
| БД | SQLCipher (rusqlite + sqlcipher feature) | SQLite + AES-256 шифрование страниц |
| UI | React + Tailwind (в WebView) | Знакомый стек |

## @-импорты детальных правил

@.claude/rules/crypto.md
@.claude/rules/security.md
@.claude/rules/vault-schema.md
@.claude/rules/browser-extension.md
@.claude/rules/testing.md
