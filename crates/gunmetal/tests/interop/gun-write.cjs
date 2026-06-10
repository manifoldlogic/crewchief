// GUN.js interop: write {name: 'Alice'} to the `test` soul via a gunmetal
// relay. Args: <relay-url> <path-to-gun.js>
// Exits 0 once the relay acknowledges the PUT.
var relayUrl = process.argv[2];
var gunPath = process.argv[3];
var Gun = require(gunPath);

var gun = Gun({
  peers: [relayUrl],
  localStorage: false,
  radisk: false,
  axe: false,
  multicast: false,
  // gun.js only auto-detects window.WebSocket; Node 22 has a global.
  WebSocket: WebSocket,
});

var done = false;
gun.get('test').put({ name: 'Alice' }, function (ack) {
  if (done) return;
  done = true;
  if (ack.err) {
    console.error('PUT_ERR', ack.err);
    process.exit(1);
  }
  console.log('PUT_ACK', JSON.stringify(ack.ok));
  setTimeout(function () { process.exit(0); }, 100);
});

setTimeout(function () {
  if (!done) {
    console.error('TIMEOUT waiting for put ack');
    process.exit(2);
  }
}, 10000);
