import assert from 'node:assert/strict';
import {
  PAIRING_TOKEN_STORAGE_KEY,
  consumeHashToken,
  maskPairingToken,
  readHashToken,
} from './pairingToken.ts';

assert.equal(PAIRING_TOKEN_STORAGE_KEY, 'server_token');

assert.equal(readHashToken('#token=AbC-_123'), 'AbC-_123');
assert.equal(readHashToken('token=AbC-_123'), 'AbC-_123');
assert.equal(readHashToken('#x=1&token=abc'), 'abc');
assert.equal(readHashToken('#/'), null);
assert.equal(readHashToken('#token='), null);
assert.equal(readHashToken(''), null);

const fakeWindow = hash => {
  const calls = { stored: new Map(), replaced: [] };
  const win = {
    location: { hash, pathname: '/', search: '' },
    history: { replaceState: (_d, _u, url) => calls.replaced.push(url) },
    localStorage: { setItem: (k, v) => calls.stored.set(k, v) },
  };
  return { win, calls };
};

{
  const { win, calls } = fakeWindow('#token=secret-1');
  assert.equal(consumeHashToken(win), true);
  assert.equal(calls.stored.get('server_token'), 'secret-1');
  assert.deepEqual(calls.replaced, ['/'], 'fragment stripped from the address bar');
}

{
  const { win, calls } = fakeWindow('#/');
  assert.equal(consumeHashToken(win), false);
  assert.equal(calls.stored.size, 0);
  assert.deepEqual(calls.replaced, [], 'normal hash routes are left alone');
}

assert.equal(
  maskPairingToken('http://192.168.1.20:8090/#token=AbC-_123'),
  'http://192.168.1.20:8090/#token=••••••',
);
assert.equal(
  maskPairingToken('android-stream-desk://connect?v=1&host=h&wsPort=8089&token=AbC-_123'),
  'android-stream-desk://connect?v=1&host=h&wsPort=8089&token=••••••',
);
assert.equal(maskPairingToken('http://192.168.1.20:8090'), 'http://192.168.1.20:8090');
