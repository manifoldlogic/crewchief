<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		Nodes are flat, so how do you model a list that many people append to at once? Arrays
		don't merge — two peers both writing <code>list[3]</code> is a fight. Gunmetal (like GUN)
		uses <strong>sets</strong> instead: a node whose keys are generated, time-sortable unique
		ids, one per item.
	</p>
	<pre><code>{`// primitives: value stored under a generated uuid key
gun.setValue('rooms/lobby/messages', JSON.stringify('hello'));

// objects: a new item node is created and linked into the set
const itemSoul = gun.setObject('todos/house', JSON.stringify({
  text: 'buy milk', done: false
}));`}</code></pre>
	<p>
		Because the generated keys (<strong>uuids</strong> — unique ids with a timestamp prefix)
		sort in creation order, rendering a chat log is just "sort the keys" — no clocks to
		coordinate, no index collisions, and two peers appending simultaneously both win.
	</p>
	<p>
		Reading a set is the node-level subscription from last chapter: <code>onNode</code> fires
		<code>(value, key)</code> per item — the value is the item itself (primitives) or a link
		to the item node (objects; subscribe to it as you discover it). Removal is
		<code>unset(setSoul, itemSoul)</code>, which nulls the <em>link</em> — the item node
		itself survives (a deliberate GUN semantic: someone else may still reference it).
	</p>
	<p>
		Filtering uses <strong>LEX</strong> (GUN's lexical match syntax): exact, prefix, and range
		matches over keys — and since set keys are time-ordered, a key range <em>is</em> a time
		range. The <a href="/gunmetal/reference/lex">lex reference</a> has the full forms.
	</p>
	<h2>Try it</h2>
	<p>
		A todo list and a chat room, each running in a single session for now. <em>Notice anyone
		could edit these if they were shared — right now nothing is protected. Chapters 8–10
		(Identity, Privacy, Permissions) are where you lock things down.</em>
	</p>
</div>
