// Pairing token shared with the Companion (S-SEC2). Stored under the same key
// in every client flavour: QR scan (APK), `#token=` fragment (web client) and
// the USB-mode bridge all write `server_token`.
export const PAIRING_TOKEN_STORAGE_KEY = 'server_token';

const TOKEN_MASK = '••••••';

export function readHashToken(hash: string): string | null {
  const fragment = hash.startsWith('#') ? hash.slice(1) : hash;
  for (const part of fragment.split('&')) {
    if (part.startsWith('token=')) {
      let token: string;
      try {
        token = decodeURIComponent(part.slice('token='.length)).trim();
      } catch {
        return null;
      }
      return token || null;
    }
  }
  return null;
}

interface HashTokenWindow {
  location: { hash: string; pathname: string; search: string };
  history: { replaceState(data: unknown, unused: string, url?: string): void };
  localStorage: { setItem(key: string, value: string): void };
}

/**
 * Moves a `#token=` fragment into localStorage and strips it from the address
 * bar. Must run before the hash router reads the location.
 */
export function consumeHashToken(win: HashTokenWindow): boolean {
  const token = readHashToken(win.location.hash);
  if (!token) return false;
  try {
    win.localStorage.setItem(PAIRING_TOKEN_STORAGE_KEY, token);
  } catch (_) {
    /* storage blocked — still strip the fragment */
  }
  win.history.replaceState(null, '', `${win.location.pathname}${win.location.search}`);
  return true;
}

/** Hides the token value in a URL or QR payload shown as text. */
export function maskPairingToken(text: string): string {
  return text.replace(/([?&#]token=)[^&#]*/g, `$1${TOKEN_MASK}`);
}
