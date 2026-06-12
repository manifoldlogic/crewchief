<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		Two peers, disconnected from each other, both edit the same key. Reconnect them. Now what?
		A central database never faces this — it serializes writes. A decentralized one faces it
		constantly, and "last write wins" is not an answer when there's no shared clock to say
		which write was last (clocks drift, and a malicious peer can claim any time it likes).
	</p>
	<p>
		Gunmetal resolves conflicts with <strong>HAM</strong> — the <em>Hypothetical Amnesia
		Machine</em>, GUN's conflict-resolution algorithm, a <strong>CRDT</strong>
		(conflict-free replicated data type: a merge rule guaranteed to produce the same result
		on every peer, in any order, without coordination). Every value carries a
		<em>state</em> number (the <code>&gt;</code> field you saw in the wire frame in chapter
		5). On conflict, roughly:
	</p>
	<ul>
		<li>Higher state wins — newer information beats older.</li>
		<li>States from the future are held back until their time comes (a drift cap defangs
			peers that lie about clocks).</li>
		<li>Equal states tie-break on the value itself (lexically) — arbitrary, but
			<em>identically arbitrary everywhere</em>, which is the property that matters.</li>
	</ul>
	<p>
		The result: every peer that has seen the same set of writes holds the same data, no
		referee required. Convergence, not correctness, is the promise — HAM picks <em>a</em>
		winner, deterministically; it doesn't know which edit was "right".
	</p>
	<h2>Try it</h2>
	<p>
		Sync the two sessions, hit <em>Disconnect both from relay</em>, type different things on
		each side, then reconnect — watch both sides land on the same value.
	</p>
</div>
