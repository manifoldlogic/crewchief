// GUN.js Node relay built from the VENDORED gun submodule — the
// comparison target for bench-relay.ts. Args: <port> <storage-dir>
// Uses the package's real Node entry (index.js → lib/server.js), which
// wires the modern WebSocket adapter (lib/wire.js), storage, SEA, AXE.
// 'ws' must be resolvable (run with NODE_PATH=web/node_modules).
// NOTE: `var`, not `const` — a TDZ'd top-level `Gun` binding poisons
// gun.js's internal `typeof Gun` probes.
var port = Number(process.argv[2] || 8770);
var file = process.argv[3] || 'gun-relay-data';

var Gun = require('../../crates/gunmetal/sources/gun/index.js');

var server = require('http').createServer(function (req, res) {
	if (req.url === '/health') {
		res.writeHead(200, { 'content-type': 'application/json' });
		res.end(JSON.stringify({ ok: true, pid: process.pid }));
		return;
	}
	res.writeHead(404);
	res.end();
});

var gun = Gun({ web: server, file: file, multicast: false });

// Upstream incompatibility shim: gun's core mesh.hear dispatches on
// `raw[0] === '['`/'{', but the modern 'ws' package delivers Buffers,
// where raw[0] is a byte — every frame is silently dropped and the
// relay is deaf. Browsers deliver strings, which is why gun works
// there. Stringify before hear.
var mesh = gun._.opt.mesh;
var hear = mesh.hear;
mesh.hear = function (raw, peer) {
	return hear.call(this, Buffer.isBuffer(raw) ? raw.toString('utf8') : raw, peer);
};

server.listen(port, function () {
	console.log('gun.js relay listening on 0.0.0.0:' + port);
});
