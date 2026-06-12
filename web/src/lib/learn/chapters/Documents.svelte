<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		Links keep nodes small and independently syncable — but sometimes you want the whole
		picture at once: a profile with its address and employer inlined, like one JSON document.
		That's <em>assembly</em>, and gunmetal does it for you:
	</p>
	<pre><code>{`gun.load('profile/ada', (json) => {
  // every link followed, the full tree as one object:
  // { name: "Ada", address: { city: "London" }, employer: { ... } }
});`}</code></pre>
	<p>
		<code>load</code> recursively follows every link from the starting node, strips the graph
		metadata, and hands you a plain object once. (Its sibling <code>open</code> does the same
		but keeps firing on every change at any depth; cycles are detected, and a depth limit is
		available — see the <a href="/gunmetal/reference/extended">extended reference</a>.)
		Deep navigation without assembly is <code>pathVal('profile/ada', 'address.city')</code>,
		which walks one dotted path to a single value.
	</p>
	<p>
		The inverse question — "is there anything here at all?" — matters for honest UIs.
		<code>notWithin(soul, key, ms)</code> resolves <code>true</code> if nothing turned up in
		time, which is how you render an empty state instead of an eternal spinner. One caveat,
		stated plainly: in a distributed system <em>absence can never be guaranteed</em> — some
		peer you haven't met may hold the data. <code>not()</code> means "nothing found here,
		yet".
	</p>
	<h2>Try it</h2>
	<p>
		Build a nested profile (the address and employer are separate linked nodes), assemble it
		with <em>Load full document</em>, then look up a profile that doesn't exist.
	</p>
</div>
