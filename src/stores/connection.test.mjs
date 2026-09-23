import assert from 'node:assert/strict';

// --- minimal browser globals the store touches at import / connect time ---
const storage = new Map();
globalThis.localStorage = {
  getItem: k => (storage.has(k) ? storage.get(k) : null),
  setItem: (k, v) => storage.set(k, String(v)),
  removeItem: k => storage.delete(k),
};
globalThis.window = globalThis;
globalThis.addEventListener = () => {};
globalThis.dispatchEvent = () => true;
globalThis.CustomEvent = class CustomEvent {
  constructor(type, init) {
    this.type = type;
    this.detail = init?.detail;
  }
};

// Fake WebSocket: `behaviour` decides what each new socket does.
let behaviour = 'silent'; // 'open' | 'error' | 'silent'
const created = [];
globalThis.WebSocket = class FakeWebSocket {
  static OPEN = 1;
  constructor(url) {
    this.url = url;
    this.readyState = 0;
    this.closed = false;
    this.sent = [];
    created.push(this);
    queueMicrotask(() => {
      if (behaviour === 'open') {
        this.readyState = FakeWebSocket.OPEN;
        this.onopen?.();
      } else if (behaviour === 'error') {
        this.onerror?.();
        this.onclose?.();
      }
    });
  }
  send(data) {
    this.sent.push(data);
  }
  close() {
    this.closed = true;
  }
};

const { createPinia, setActivePinia } = await import('pinia');
const { isLoopbackHost, probeWebSocket, useConnectionStore } = await import('./connection.ts');

// ---------------------------------------------------------------- isLoopbackHost
for (const h of ['127.0.0.1', 'localhost', 'LOCALHOST', ' 127.0.0.1 ', '::1', '[::1]']) {
  assert.equal(isLoopbackHost(h), true, `${h} is loopback`);
}
for (const h of ['10.0.0.5', '192.168.1.20', '127.0.0.2', '', 'example.com']) {
  assert.equal(isLoopbackHost(h), false, `${h} is not loopback`);
}

// ---------------------------------------------------------------- probeWebSocket
behaviour = 'open';
assert.equal(await probeWebSocket('ws://127.0.0.1:8089', 200), true, 'open handshake → true');
assert.equal(created.at(-1).closed, true, 'probe socket is closed after success');

behaviour = 'error';
assert.equal(await probeWebSocket('ws://127.0.0.1:8089', 200), false, 'error → false');

behaviour = 'silent';
assert.equal(await probeWebSocket('ws://127.0.0.1:8089', 30), false, 'no answer → timeout → false');
assert.equal(created.at(-1).closed, true, 'timed-out probe socket is closed');

// ---------------------------------------------------------------- online gate
// USB mode: with the WebView reporting offline (Wi-Fi off), a loopback
// target must still be attempted; a LAN target must not.
setActivePinia(createPinia());
let store = useConnectionStore();
store.isOnline = false;
store.port = '8089';

created.length = 0;
behaviour = 'silent';
store.ipAddress = '10.0.0.5';
store.connect();
assert.equal(store.status, 'disconnected', 'LAN target offline → skipped');
assert.equal(created.length, 0, 'LAN target offline → no socket opened');

store.ipAddress = '127.0.0.1';
store.connect();
assert.equal(store.status, 'connecting', 'loopback target offline → still attempted');
assert.equal(created.length, 1, 'loopback target offline → socket opened');
assert.equal(created[0].url, 'ws://127.0.0.1:8089');
store.disconnect();

// ---------------------------------------------------------------- tryLoopbackAutoConnect
setActivePinia(createPinia());
storage.clear();
store = useConnectionStore();
store.isOnline = true;

store.ipAddress = '192.168.1.20';
behaviour = 'open';
created.length = 0;
assert.equal(await store.tryLoopbackAutoConnect(), false, 'saved address → no probe');
assert.equal(created.length, 0, 'saved address → no socket opened');

store.ipAddress = '';
behaviour = 'error';
assert.equal(await store.tryLoopbackAutoConnect(), false, 'no Companion on loopback → false');
assert.equal(store.ipAddress, '', 'address stays empty when probe fails');

behaviour = 'open';
created.length = 0;
assert.equal(await store.tryLoopbackAutoConnect(), true, 'Companion answers on loopback → adopted');
assert.equal(store.ipAddress, '127.0.0.1');
assert.equal(storage.get('server_ip'), '127.0.0.1', 'adopted address is persisted');
assert.ok(created.some(ws => ws.url === 'ws://127.0.0.1:8089'), 'real connect opened to loopback');
store.disconnect();

// ------------------------------------------------- sendOrQueue / revive
const types = ws => ws.sent.map(d => JSON.parse(d).type);
const tick = () => new Promise(r => setTimeout(r, 0));

// Open socket: press goes straight out.
setActivePinia(createPinia());
storage.clear();
store = useConnectionStore();
store.isOnline = true;
store.ipAddress = '127.0.0.1';
store.port = '8089';
behaviour = 'open';
created.length = 0;
store.connect();
await tick();
const live = created.at(-1);
assert.equal(store.status, 'connected', 'socket opened');
assert.equal(store.sendOrQueue({ type: 'press', payload: { id: 'a' } }), true, 'open socket → sent');
assert.ok(types(live).includes('press'), 'press written to the socket');

// Socket stale (screen was off): press is queued, not lost, and replayed on
// the next open socket — this is the double-tap bug.
live.readyState = 0;
const beforeCount = created.length;
assert.equal(
  store.sendOrQueue({ type: 'press', payload: { id: 'queued' } }),
  false,
  'stale socket → reports not sent',
);
assert.ok(created.length > beforeCount, 'stale socket triggers an immediate reconnect');
await tick();
const revived = created.at(-1);
const replayed = revived.sent.map(d => JSON.parse(d)).filter(m => m.type === 'press');
assert.equal(replayed.length, 1, 'queued press replayed exactly once on reopen');
assert.equal(replayed[0].payload.id, 'queued', 'the queued press is the one replayed');

// A press older than the TTL is dropped rather than fired late.
store.disconnect();
setActivePinia(createPinia());
store = useConnectionStore();
store.isOnline = true;
store.ipAddress = '127.0.0.1';
store.port = '8089';
behaviour = 'silent';
created.length = 0;
store.connect();
await tick();
const deadSock = created.at(-1);
deadSock.readyState = 0;
store.sendOrQueue({ type: 'press', payload: { id: 'stale' } });
// Backdate beyond the 5s TTL by replaying through a fresh open socket later.
const realNow = Date.now;
Date.now = () => realNow() + 6000;
behaviour = 'open';
store.connect();
await tick();
Date.now = realNow;
const afterTtl = created.at(-1);
assert.equal(
  afterTtl.sent.map(d => JSON.parse(d).type).filter(t => t === 'press').length,
  0,
  'press older than the TTL is dropped, not fired late',
);
store.disconnect();

// revive() on a healthy socket probes it instead of tearing it down.
setActivePinia(createPinia());
store = useConnectionStore();
store.isOnline = true;
store.ipAddress = '127.0.0.1';
store.port = '8089';
behaviour = 'open';
created.length = 0;
store.connect();
await tick();
const healthy = created.at(-1);
const countBeforeRevive = created.length;
healthy.sent.length = 0;
store.revive();
assert.equal(healthy.closed, false, 'healthy socket is not torn down by revive()');
assert.equal(created.length, countBeforeRevive, 'healthy socket: no new socket opened');
assert.ok(types(healthy).includes('ping'), 'revive() probes the socket with a ping');
store.disconnect();

console.log('✅ connection store tests passed');
process.exit(0);
