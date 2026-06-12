<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		One promise has been implicit since chapter 0: gunmetal isn't GUN-<em>like</em>, it is
		<strong>on the same mesh</strong>. Same wire format, same DAM routing, same HAM merge
		rules, same SEA crypto surface, same RAD storage format. A GUN.js browser client and a
		gunmetal wasm client connected to the same relay are just… peers.
	</p>
	<p>
		The demo below is that claim, running: the left session is the real, unmodified GUN.js
		library; the right is gunmetal compiled to WebAssembly. The relay between them is the
		Rust binary you ran in chapter 5. Type in either box.
	</p>
	<p>
		This is also this site's most important test — every capability you've learned is
		exercised against the reference implementation on every CI run. Interop isn't a feature
		here; it's the definition of correctness.
	</p>
	<h2>You made it</h2>
	<p>
		You now know the graph model, reactivity, collections, documents, sync, conflict
		resolution, persistence, identity, privacy, and permissions. Where to go next:
	</p>
	<ul>
		<li><a href="/gunmetal/quickstart">Quickstart</a> — put gunmetal in your own project (Rust or browser).</li>
		<li><a href="/gunmetal/demos">The demo catalog</a> — every pattern on this site, with annotated source.</li>
		<li><a href="/gunmetal/reference">API reference</a> — module-by-module contracts.</li>
		<li><a href="/gunmetal/demos/wire-inspector">The wire inspector</a> — keep it open while you build; it answers "why isn't my put syncing?" faster than anything else.</li>
	</ul>
</div>
