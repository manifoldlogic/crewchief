<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		Everything so far lived in memory — reload the page and it's gone. Persistence makes the
		graph durable on each peer, which is also what makes <em>offline-first</em> apps work:
		write locally, sync whenever a connection happens to exist.
	</p>
	<pre><code>{`await gun.enablePersistence('my-app');  // IndexedDB, namespaced by name
// existing data hydrates through the normal merge path,
// then every accepted write persists automatically`}</code></pre>
	<p>
		In the browser that's <strong>IndexedDB</strong> (the browser's built-in database).
		Hydration replays stored values <em>with their original HAM states</em> through the same
		merge path a network message takes — so local history and remote updates reconcile by the
		chapter-6 rules, never by overwriting each other.
	</p>
	<p>
		On servers and relays the engine is <strong>RAD</strong> (the Radix storage engine,
		a.k.a. radisk): values live in a radix tree, serialized into chunked files in a
		<code>radata/</code> directory, batched every 250&nbsp;ms, written atomically. The format
		is JSON-compatible with GUN.js's own data directories. You don't call RAD directly — the
		relay you ran in chapter 5 was already persisting through it; details live in the
		<a href="/gunmetal/reference/rad">rad reference</a>.
	</p>
	<p>
		One sharp edge worth knowing: persistence is <em>per name</em>. Two apps (or two iframes)
		on the same origin that use the same database name share data. The demo below gives each
		session its own name — that separation is what makes its offline behavior honest.
	</p>
	<h2>Try it</h2>
	<p>
		Disconnect, edit, reconnect — then imagine the reload: the demo's e2e test reloads a
		session against an unreachable relay and the data is still there, served from its own
		IndexedDB.
	</p>
</div>
