/**
 * Relay load benchmark: gunmetal-relay vs the vendored GUN.js Node relay
 * (spec: _SPECS/crewchief/spec/gunmetal-bench.md, tier 1).
 *
 * The load generator (bench-driver.cjs) uses REAL gun.js clients and is
 * byte-identical for both targets — generator overhead cancels out, so
 * the deltas isolate the relay. Subscribers .on() the bench soul; writers
 * send paced puts whose value carries (seq, sentAt). Reported per target:
 * achieved throughput, put→ack latency, put→subscriber fan-out latency,
 * and relay peak RSS.
 *
 *   bun scripts/bench-relay.ts compare            # both targets, table
 *   bun scripts/bench-relay.ts gunmetal|gun       # one target
 *   flags: --subs 10 --writers 4 --rate 500 --duration 15 --warmup 5
 *
 * Run on an otherwise-idle machine; results land in bench-results/.
 */

import { mkdtempSync, mkdirSync, rmSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn, execSync, type ChildProcess } from 'node:child_process';

interface Options {
	subs: number;
	writers: number;
	rate: number;
	duration: number;
	warmup: number;
	port: number;
}

interface DriverResult {
	achievedPutsPerSec: number;
	acksPerSec: number;
	ackLatencyMs: { p50: number; p95: number; p99: number } | null;
	fanoutLatencyMs: { p50: number; p95: number; p99: number } | null;
	fanoutFiresPerSec: number;
}

interface TargetResult extends DriverResult {
	target: string;
	offeredPutsPerSec: number;
	relayPeakRssMb: number;
	connections: number;
}

function parseArgs(): { mode: string; opts: Options } {
	const args = process.argv.slice(2);
	const mode = args.find((a) => !a.startsWith('--')) ?? 'compare';
	const flag = (name: string, fallback: number) => {
		const i = args.indexOf(`--${name}`);
		return i !== -1 ? Number(args[i + 1]) : fallback;
	};
	return {
		mode,
		opts: {
			subs: flag('subs', 10),
			writers: flag('writers', 4),
			rate: flag('rate', 500),
			duration: flag('duration', 15),
			warmup: flag('warmup', 5),
			port: flag('port', 8772)
		}
	};
}

async function waitForHealth(url: string, timeoutMs: number): Promise<void> {
	const start = Date.now();
	while (Date.now() - start < timeoutMs) {
		try {
			const res = await fetch(url);
			if (res.ok) return;
		} catch {
			/* not up yet */
		}
		await Bun.sleep(250);
	}
	throw new Error(`relay never became healthy: ${url}`);
}

function spawnTarget(target: string, port: number, dataDir: string): ChildProcess {
	if (target === 'gunmetal') {
		const bin = join(import.meta.dir, '../../target/release/gunmetal-relay');
		if (!existsSync(bin)) {
			console.log('building gunmetal-relay (release — never bench debug builds)...');
			execSync('cargo build --release -p gunmetal --features relay --bin gunmetal-relay', {
				cwd: join(import.meta.dir, '../..'),
				stdio: 'inherit'
			});
		}
		return spawn(bin, ['--port', String(port), '--file', dataDir], { stdio: 'ignore' });
	}
	if (target === 'gun') {
		return spawn('node', [join(import.meta.dir, 'gun-relay.cjs'), String(port), dataDir], {
			stdio: 'ignore',
			env: { ...process.env, NODE_PATH: join(import.meta.dir, '../node_modules') }
		});
	}
	throw new Error(`unknown target: ${target}`);
}

function sampleRssMb(pid: number): number {
	try {
		return Number(execSync(`ps -o rss= -p ${pid}`).toString().trim()) / 1024;
	} catch {
		return 0;
	}
}

async function benchTarget(target: string, opts: Options): Promise<TargetResult> {
	const dataDir = mkdtempSync(join(tmpdir(), `bench-${target}-`));
	const relay = spawnTarget(target, opts.port, dataDir);
	try {
		await waitForHealth(`http://localhost:${opts.port}/health`, 60_000);

		let peakRss = 0;
		const rssTimer = setInterval(() => {
			if (relay.pid) peakRss = Math.max(peakRss, sampleRssMb(relay.pid));
		}, 1000);

		const driver = spawn(
			'node',
			[
				join(import.meta.dir, 'bench-driver.cjs'),
				`ws://localhost:${opts.port}/gun`,
				String(opts.subs),
				String(opts.writers),
				String(opts.rate),
				String(opts.warmup),
				String(opts.duration)
			],
			{ stdio: ['ignore', 'pipe', 'inherit'] }
		);
		let stdout = '';
		driver.stdout!.on('data', (chunk) => (stdout += chunk));
		const exitCode: number = await new Promise((resolve) => driver.once('exit', resolve));
		clearInterval(rssTimer);
		if (exitCode !== 0) throw new Error(`driver exited ${exitCode} for ${target}`);

		const line = stdout.split('\n').find((l) => l.startsWith('RESULT '));
		if (!line) throw new Error(`driver produced no RESULT line for ${target}`);
		const driverResult: DriverResult = JSON.parse(line.slice('RESULT '.length));

		return {
			target,
			offeredPutsPerSec: opts.rate,
			relayPeakRssMb: Math.round(peakRss * 10) / 10,
			connections: opts.subs + opts.writers,
			...driverResult
		};
	} finally {
		const exited = new Promise<void>((resolve) => {
			relay.once('exit', () => resolve());
			setTimeout(resolve, 5000);
		});
		relay.kill();
		await exited;
		rmSync(dataDir, { recursive: true, force: true });
	}
}

function printTable(results: TargetResult[]) {
	const row = (label: string, fn: (r: TargetResult) => string) =>
		console.log(label.padEnd(26) + results.map((r) => fn(r).padStart(16)).join(''));
	const latency = (p: TargetResult['ackLatencyMs']) =>
		p ? `${p.p50}/${p.p95}/${p.p99}` : 'n/a';
	console.log();
	row('', (r) => r.target);
	row('connections', (r) => String(r.connections));
	row('offered puts/s', (r) => String(r.offeredPutsPerSec));
	row('achieved puts/s', (r) => String(r.achievedPutsPerSec));
	row('acks/s', (r) => String(r.acksPerSec));
	row('ack p50/p95/p99 (ms)', (r) => latency(r.ackLatencyMs));
	row('fan-out p50/p95/p99 (ms)', (r) => latency(r.fanoutLatencyMs));
	row('fan-out fires/s', (r) => String(r.fanoutFiresPerSec));
	row('relay peak RSS (MB)', (r) => String(r.relayPeakRssMb));
	console.log();
}

const { mode, opts } = parseArgs();
const targets = mode === 'compare' ? ['gunmetal', 'gun'] : [mode];
const results: TargetResult[] = [];
for (const [index, target] of targets.entries()) {
	// Distinct port per target: never race the previous relay's shutdown.
	const targetOpts = { ...opts, port: opts.port + index };
	console.log(
		`benchmarking ${target} on :${targetOpts.port} (${opts.subs} subs, ` +
			`${opts.writers} writers, ${opts.rate} puts/s offered, ` +
			`${opts.warmup}s warmup + ${opts.duration}s measure)...`
	);
	results.push(await benchTarget(target, targetOpts));
}
printTable(results);

mkdirSync(join(import.meta.dir, '../bench-results'), { recursive: true });
const stamp = new Date().toISOString().replace(/[:.]/g, '-');
const outPath = join(import.meta.dir, `../bench-results/relay-${stamp}.json`);
writeFileSync(outPath, JSON.stringify({ opts, results }, null, 2));
console.log(`results written to ${outPath}`);
