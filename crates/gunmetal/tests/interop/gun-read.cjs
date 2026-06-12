// GUN.js interop: read the `test` soul from a gunmetal relay and print it.
// Args: <relay-url> <path-to-gun.js>
// Prints READ {"name":...,"age":...} and exits 0 when both fields arrive.
var relayUrl = process.argv[2];
var gunPath = process.argv[3];
var Gun = require(gunPath);

var gun = Gun({
  peers: [relayUrl],
  localStorage: false,
  radisk: false,
  axe: false,
  multicast: false,
  WebSocket: WebSocket,
});

var done = false;
// .on() so late-arriving fields (relayed from other peers) still resolve.
gun.get('test').on(function (data) {
  if (done) return;
  if (data && data.name !== undefined && data.age !== undefined) {
    done = true;
    console.log('READ', JSON.stringify({ name: data.name, age: data.age }));
    setTimeout(function () { process.exit(0); }, 100);
  }
});

setTimeout(function () {
  if (!done) {
    console.error('TIMEOUT waiting for data');
    process.exit(2);
  }
}, 10000);
