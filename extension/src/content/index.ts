// Content script — isolated world, IIFE format, no ES module imports at runtime.
// Receives FILL messages from the popup (via background) and autofills
// the first visible username + password fields on the page.

const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
  HTMLInputElement.prototype,
  "value"
)?.set;

function isVisible(el: HTMLElement): boolean {
  return (
    el.offsetParent !== null &&
    getComputedStyle(el).visibility !== "hidden" &&
    getComputedStyle(el).opacity !== "0" &&
    el.getBoundingClientRect().width > 0 &&
    el.getBoundingClientRect().height > 0
  );
}

function fillInput(input: HTMLInputElement, value: string): void {
  nativeInputValueSetter?.call(input, value);
  // Prevent browser from saving this value in its password manager
  input.setAttribute("autocomplete", "new-password");
  // Notify React/Vue/Angular that the value changed
  input.dispatchEvent(new Event("input", { bubbles: true }));
  input.dispatchEvent(new Event("change", { bubbles: true }));
}

function fillPage(username: string, password: string): void {
  const inputs = Array.from(document.querySelectorAll<HTMLInputElement>("input"));
  let filledUser = false;
  let filledPass = false;

  for (const input of inputs) {
    if (!isVisible(input)) continue;

    if (!filledUser && (input.type === "text" || input.type === "email")) {
      fillInput(input, username);
      filledUser = true;
    } else if (!filledPass && input.type === "password") {
      fillInput(input, password);
      filledPass = true;
    }

    if (filledUser && filledPass) break;
  }
}

// Listen for FILL messages from the popup (relayed through background)
chrome.runtime.onMessage.addListener(
  (msg: { type: string; username?: string; password?: string }) => {
    if (msg.type === "FILL" && msg.username !== undefined && msg.password !== undefined) {
      fillPage(msg.username, msg.password);
    }
  }
);
