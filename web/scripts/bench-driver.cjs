// Load driver for bench-relay.ts: real gun.js clients (the vendored
// submodule) generate IDENTICAL load against whichever relay URL they're
// pointed at — generator overhead cancels out across targets, so the
// delta isolates the relay. Prints a JSON result line to stdout.
//
// args: <relay-url> <subs> <writers> <rate/s> <warmup-s> <duration-s>
var relayUrl = process.argv[2];
function reqNum(name, value, min) {
	if (!Number.isFinite(value) || value < min) {
		console.error('bench-driver: --' + name + ' must be a finite number >= ' + min + ' (got ' + value + ')');
		process.exit(1);
	}
	return value;
}
// subs/writers/rate/duration must be >= 1 (writers.length === 0 crashes the
// load loop; duration 0 divides by zero in the metrics). warmup may be 0.
var SUBS = reqNum('subs', Number(process.argv[3] || 10), 1);
var WRITERS = reqNum('writers', Number(process.argv[4] || 4), 1);
var RATE = reqNum('rate', Number(process.argv[5] || 500), 1);
var WARMUP_S = reqNum('warmup', Number(process.argv[6] || 5), 0);
var DURATION_S = reqNum('duration', Number(process.argv[7] || 15), 1);

var Gun = require('../../crates/gunmetal/sources/gun/gun.js');

var SOUL = 'bench/throughput';
var KEY = 'k';
var PROBES = Math.min(3, SUBS);

function mkClient() {
	return Gun({
		peers: [relayUrl],
		localStorage: false,
		radisk: false,
		axe: false,
		multicast: false,
		WebSocket: WebSocket // Node 22 global
	});
}

var measuring = false;
var sent = 0;
var acked = 0;
var ackLatencies = [];
var fanoutLatencies = [];
var fanoutFires = 0;

// ── subscribers ──────────────────────────────────────────────────────
for (var s = 0; s < SUBS; s++) {
	(function (index) {
		var client = mkClient();
		client.get(SOUL).get(KEY).on(function (value) {
			if (!measuring) return;
			fanoutFires++;
			if (index < PROBES && typeof value === 'string' && value.charAt(0) === '{') {
				try {
					var payload = JSON.parse(value);
					if (typeof payload.t === 'number') fanoutLatencies.push(Date.now() - payload.t);
				} catch (e) { /* ignore */ }
			}
		});
	})(s);
}

// ── writers ──────────────────────────────────────────────────────────
var writers = [];
for (var w = 0; w < WRITERS; w++) writers.push(mkClient());

var seq = 0;
var TICK_MS = 20;
var perTick = Math.max(1, Math.round((RATE * TICK_MS) / 1000));
var ticker = setInterval(function () {
	for (var i = 0; i < perTick; i++) {
		var writer = writers[seq % writers.length];
		var now = Date.now();
		var value = JSON.stringify({ seq: seq, t: now });
		(function (sentAt, isMeasured) {
			writer.get(SOUL).get(KEY).put(value, function (ack) {
				if (!isMeasured || ack.err) return;
				acked++;
				ackLatencies.push(Date.now() - sentAt);
			});
		})(now, measuring);
		if (measuring) sent++;
		seq++;
	}
}, TICK_MS);

function percentiles(values) {
	if (!values.length) return null;
	var sorted = values.slice().sort(function (a, b) { return a - b; });
	var pick = function (p) {
		return sorted[Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length))];
	};
	return { p50: pick(50), p95: pick(95), p99: pick(99) };
}

setTimeout(function () {
	measuring = true;
	setTimeout(function () {
		measuring = false;
		// let in-flight acks/fan-out land
		setTimeout(function () {
			clearInterval(ticker);
			console.log(
				'RESULT ' +
					JSON.stringify({
						achievedPutsPerSec: Math.round(sent / DURATION_S),
						acksPerSec: Math.round(acked / DURATION_S),
						ackLatencyMs: percentiles(ackLatencies),
						fanoutLatencyMs: percentiles(fanoutLatencies),
						fanoutFiresPerSec: Math.round(fanoutFires / DURATION_S)
					})
			);
			process.exit(0);
		}, 1500);
	}, DURATION_S * 1000);
}, WARMUP_S * 1000);
