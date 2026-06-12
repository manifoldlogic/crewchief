<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		Everything so far ran inside one browser tab. Sync is what makes it a database: a
		<strong>peer</strong> is any other running gunmetal (or GUN.js) instance — another tab,
		someone else's laptop, a server — and peers exchange their writes.
	</p>
	<h2>"Why is there a server in a decentralized system?"</h2>
	<p>
		Fair question. Browsers can't accept incoming connections, so two browser tabs on
		different networks have no way to find or reach each other directly. A
		<strong>relay</strong> is the meeting point: a peer that listens on a public address and
		forwards messages between everyone connected to it. Crucially, a relay is <em>not an
		authority</em> — it doesn't decide what's true (the merge rules in the next chapter do),
		it can't forge your signed data (chapter 8), and any number of relays can serve the same
		mesh. Run one yourself:
	</p>
	<pre><code>{`cargo run -p gunmetal --features relay --bin gunmetal-relay -- --port 8765`}</code></pre>
	<pre><code>{`const gun = WasmGun.withOptions(JSON.stringify({ localStorage: false }));
gun.connect('ws://localhost:8765/gun');`}</code></pre>
	<h2>What actually travels</h2>
	<p>
		A message is just JSON describing a change. When you put, roughly this leaves your
		machine:
	</p>
	<pre><code>{`{"#":"msg-id","put":{"people/ada":{"_":{"#":"people/ada",">":{"name":1718...}},"name":"Ada"}}}`}</code></pre>
	<p>
		— the soul, the changed key, the value, and a timestamp-ish <em>state</em> the merge rules
		use. Every connected peer receives it, merges it, and fires the same subscriptions you
		wrote in chapter 2. The routing layer that keeps messages from looping forever is called
		<strong>DAM</strong> (Daisy-chain Ad-hoc Mesh); its mechanics — deduplication, batching,
		acknowledgments — live in the <a href="/gunmetal/reference/mesh">mesh reference</a> and
		the <a href="/gunmetal/demos/wire-inspector">wire inspector demo</a> when you want to see
		frames live.
	</p>
	<h2>Try it</h2>
	<p>
		Two iframes below are two genuinely separate sessions syncing through a relay. Type in
		either. The presence demo shows peers arriving and leaving. <em>And the standing caveat:
		anyone on this mesh can edit these souls — Identity and Permissions (chapters 8 and 10)
		are how you lock writes down.</em>
	</p>
</div>
