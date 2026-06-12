<script lang="ts">
	import * as Tabs from '$lib/components/ui/tabs';
</script>

<svelte:head><title>Quickstart — Gunmetal</title></svelte:head>

<h1 class="text-3xl font-bold tracking-tight">Get started</h1>
<p class="mt-3 max-w-2xl text-muted-foreground">
	Gunmetal lives in this repository and is not yet published to crates.io or npm — both paths
	below build from the repo.
</p>

<Tabs.Root value="rust" class="mt-8">
	<Tabs.List>
		<Tabs.Trigger value="rust" data-testid="qs-tab-rust">Rust crate</Tabs.Trigger>
		<Tabs.Trigger value="browser" data-testid="qs-tab-browser">Browser / wasm</Tabs.Trigger>
	</Tabs.List>
	<Tabs.Content value="rust">
		<div class="prose prose-neutral mt-4 dark:prose-invert">
			<pre><code>{`# Cargo.toml (path or git dependency while unpublished)
[dependencies]
gunmetal = { path = "crates/gunmetal" }`}</code></pre>
			<pre><code>{`use gunmetal::{Gun, GunOptions, GunValue};

let gun = Gun::new(GunOptions::default());
gun.get("mark").put_kv("name", GunValue::Text("Mark".into()));
assert_eq!(
    gun.get("mark").get("name").val(),
    Some(GunValue::Text("Mark".into()))
);`}</code></pre>
			<p>Run a relay other peers (including GUN.js browsers) can connect to:</p>
			<pre><code>{`cargo run -p gunmetal --features relay --bin gunmetal-relay -- --port 8765`}</code></pre>
		</div>
	</Tabs.Content>
	<Tabs.Content value="browser">
		<div class="prose prose-neutral mt-4 dark:prose-invert">
			<p>
				Build the wasm bundle from the repo (pinned to <code>wasm-bindgen 0.2.108</code> — the
				CLI version must match the workspace lockfile):
			</p>
			<pre><code>{`cargo build --target wasm32-unknown-unknown -p gunmetal --features wasm --release
wasm-bindgen --target web --out-dir pkg \\
  target/wasm32-unknown-unknown/release/gunmetal.wasm`}</code></pre>
			<p>Then use it:</p>
			<pre><code>{`import init, { WasmGun } from './pkg/gunmetal.js';

await init();
const gun = WasmGun.withOptions(JSON.stringify({ localStorage: false }));
gun.connect('ws://localhost:8765/gun');
gun.on('people/ada', 'name', (json) => console.log(JSON.parse(json)));
gun.putText('people/ada', 'name', 'Ada');`}</code></pre>
			<p>
				Every demo page's annotated source shows the same calls in context — the
				<a href="/gunmetal/demos/shared-input">shared input</a> is the smallest complete
				example.
			</p>
		</div>
	</Tabs.Content>
</Tabs.Root>
