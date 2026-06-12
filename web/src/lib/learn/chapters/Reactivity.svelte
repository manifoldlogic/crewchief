<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		Reading once is the exception in a synchronized app — usually you want to know about every
		change, whoever made it. That's a <strong>subscription</strong>:
	</p>
	<pre><code>{`const id = gun.on('people/ada', 'name', (value, key) => {
  console.log('name is now', JSON.parse(value));
});
// later: gun.off('people/ada', 'name', id);`}</code></pre>
	<p>
		The callback fires when this session writes the key, and — once you're connected to peers
		(chapter 5) — when <em>anyone anywhere</em> writes it. UI code stays identical either way:
		render from the callback and the app is collaborative for free.
	</p>
	<p>
		You can also subscribe to a whole node: <code>gun.onNode(soul, cb)</code> fires once
		<em>per changed key</em>, with <code>(value, key)</code> — handy for records whose keys you
		don't know in advance (you'll lean on this in the next chapter). One-shot reads exist too:
		<code>gun.once(soul, key)</code> returns a promise of the current value.
	</p>
	<p>
		Two practical rules: subscriptions deliver the <em>current</em> value only when something
		writes — after subscribing, read existing state explicitly (<code>get</code>/<code>getNode</code>)
		if the data may already be there; and callbacks are allowed to write and to subscribe to
		more things — chains of "discover, then watch" are the normal way to consume linked data.
	</p>
	<h2>Try it</h2>
	<p>
		Same explorer as last chapter — but now watch the <em>subscription log</em> on the left
		while you put values. Every line is a callback fire.
	</p>
</div>
