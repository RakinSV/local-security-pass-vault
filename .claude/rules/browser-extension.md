# Правила браузерного расширения

## Manifest V3 — обязательные настройки безопасности

```json
// extension/manifest.json
{
  "manifest_version": 3,
  "name": "VaultPass",
  "permissions": [
    "activeTab",          // только текущая вкладка, не все
    "nativeMessaging",    // IPC с десктопом
    "clipboardWrite"      // запись в буфер обмена
    // НЕ добавлять: "tabs", "history", "bookmarks", "<all_urls>" без необходимости
  ],
  "host_permissions": [],  // пустой — расширение не делает сетевых запросов
  
  "content_security_policy": {
    "extension_pages": "default-src 'self'; script-src 'self'; object-src 'none'; style-src 'self' 'unsafe-inline'"
    // 'unsafe-inline' для стилей — допустимо, НО:
    // script-src 'self' — НИКАКОГО eval(), НИКАКИХ inline скриптов
  },
  
  "background": {
    "service_worker": "background.js"
    // Service Worker живёт max 5 минут — принудительно очищает память с паролями
  }
}
```

## Правило: ноль сетевых запросов

```typescript
// В расширении ЗАПРЕЩЕНЫ:
// - fetch()
// - XMLHttpRequest  
// - WebSocket
// - chrome.runtime.sendMessage к внешним расширениям
// - importScripts() из внешних URL

// ВСЁ взаимодействие — только через Native Messaging к десктопу:
chrome.runtime.connectNative('com.vaultpass.native');
```

## Автозаполнение — правила безопасности

### Сравнение доменов (СТРОГО по eTLD+1)
```typescript
import { parse } from 'tldts';  // библиотека publicsuffix.org

function domainsMatch(vaultUrl: string, pageUrl: string): boolean {
    const vaultDomain = parse(vaultUrl).domain;   // "google.com" из "accounts.google.com"
    const pageDomain  = parse(pageUrl).domain;    // "google.com" из "mail.google.com"
    
    if (!vaultDomain || !pageDomain) return false;
    return vaultDomain === pageDomain;
    
    // ПРИМЕРЫ:
    // google.com vs accounts.google.com  → MATCH (поддомены OK)
    // google.com vs google.com.evil.ru   → NO MATCH
    // paypal.com vs paypa1.com           → NO MATCH
    // github.com vs github.io            → NO MATCH (разные eTLD+1)
}
```

### Заполнять ТОЛЬКО видимые поля
```typescript
function isVisible(element: HTMLElement): boolean {
    // Проверяем что поле реально видно пользователю
    return (
        element.offsetParent !== null &&           // не display:none
        getComputedStyle(element).visibility !== 'hidden' &&
        getComputedStyle(element).opacity !== '0' &&
        element.getBoundingClientRect().width > 0 &&
        element.getBoundingClientRect().height > 0
    );
}

// Автозаполнение:
const inputs = document.querySelectorAll('input[type="password"], input[type="text"]');
for (const input of inputs) {
    if (!isVisible(input as HTMLElement)) continue;  // пропускаем скрытые
    fillInput(input as HTMLInputElement, value);
}
```

### Programmatic fill (не попадает в autocomplete историю браузера)
```typescript
function fillInput(input: HTMLInputElement, value: string): void {
    // Используем нативный setter — браузер не индексирует такие значения
    const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
        HTMLInputElement.prototype, 'value'
    )?.set;
    
    nativeInputValueSetter?.call(input, value);
    
    // Триггерим события чтобы React/Vue/Angular подхватили значение
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
    
    // Устанавливаем autocomplete=off чтобы браузер не запомнил
    input.setAttribute('autocomplete', 'new-password');
}
```

## Кеширование паролей — ЗАПРЕЩЕНО

```typescript
// ЗАПРЕЩЕНО в любом месте расширения:
// const cachedPasswords = new Map<string, string>();  // НЕТ
// chrome.storage.session.set({ passwords: ... });     // НЕТ
// localStorage.setItem('pass', ...);                  // НЕТ

// ПРАВИЛЬНО: каждый запрос автозаполнения = новый IPC к десктопу
// Небольшая задержка (50-100ms) — приемлемый компромисс для безопасности

async function getPassword(itemId: string): Promise<string> {
    const response = await nativePort.sendMessage({
        action: 'get_password',
        item_id: itemId,
        nonce: crypto.randomUUID()  // одноразовый токен против replay
    });
    return response.password;
    // password живёт только в этой функции, не сохраняется
}
```

## Иконка расширения — всегда одинакова

```typescript
// ЗАПРЕЩЕНО менять иконку в зависимости от наличия совпадения:
// chrome.action.setIcon({ path: 'icon-active.png' });  // НЕТ — timing side channel!

// Иконка всегда одинакова.
// Индикатор наличия записей — ТОЛЬКО внутри popup (который открывается по клику).
// Страница не может определить есть ли запись для неё без явного открытия popup.
```

## Popup — защита от clickjacking

```typescript
// Popup расширения нельзя встроить в iframe — браузер это запрещает.
// Дополнительно в popup.html:
// <meta http-equiv="X-Frame-Options" content="DENY">
// <meta http-equiv="Content-Security-Policy" content="frame-ancestors 'none'">
```

## Изоляция content script

```typescript
// Content script выполняется в isolated world (MV3 по умолчанию).
// Страница НЕ МОЖЕТ:
// - Читать переменные content script
// - Вызывать функции content script
// - Обращаться к chrome.* API

// НО страница МОЖЕТ:
// - Загрязнять Object.prototype (prototype pollution)

// Защита — использовать Object.create(null) для хранения данных:
const safeDict: Record<string, string> = Object.create(null);
// Не наследует от Object.prototype → загрязнение не влияет
```

## Native Messaging — IPC протокол

```typescript
// Структура запроса (расширение → десктоп):
interface NativeRequest {
    id: string;          // UUID — одноразовый нonce против replay
    action: 'search' | 'get_password' | 'lock' | 'status';
    payload?: unknown;   // зашифрован публичным ключом десктопа
}

// Структура ответа (десктоп → расширение):
interface NativeResponse {
    id: string;          // тот же UUID что в запросе
    success: boolean;
    data?: unknown;      // зашифрован, подписан Ed25519
    signature: string;   // Ed25519 подпись (id + data)
}

// Верификация каждого ответа:
async function verifyResponse(response: NativeResponse): Promise<boolean> {
    const key = await getDesktopPublicKey();  // вшит при сопряжении
    return await crypto.subtle.verify(
        'Ed25519',
        key,
        hexToBuffer(response.signature),
        textEncoder.encode(response.id + JSON.stringify(response.data))
    );
}
```

## Что проверять в code review расширения

```
☐ Нет fetch() / XHR / WebSocket
☐ Нет кеширования паролей (Map, localStorage, sessionStorage, chrome.storage)
☐ Иконка не меняется при наличии совпадения
☐ Домен сравнивается через eTLD+1 (tldts), не string.includes()
☐ Заполнение только через nativeInputValueSetter
☐ Проверка видимости поля перед заполнением
☐ Каждый IPC запрос содержит уникальный nonce
☐ Ответ от десктопа верифицируется по Ed25519 подписи
☐ CSP в manifest не содержит 'unsafe-eval' или внешних URL в script-src
```
