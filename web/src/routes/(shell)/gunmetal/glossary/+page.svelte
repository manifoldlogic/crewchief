<script lang="ts">
	const terms = [
		['Soul', 'The globally-unique id of a node in the graph. Every node has exactly one.'],
		['Node', 'A flat record of key→value pairs (values may link to other nodes), plus state metadata.'],
		['Chain', 'The fluent API context produced by .get() calls — a cursor into the graph.'],
		['Relay', 'A server peers sync through. Not an authority: it routes messages and optionally persists data. Decentralized systems still use relays as meeting points.'],
		['HAM', 'Hypothetical Amnesia Machine — GUN’s CRDT conflict-resolution algorithm. Decides which concurrent write wins, deterministically, on every peer.'],
		['CRDT', 'Conflict-free Replicated Data Type — data structures that merge concurrent edits without coordination.'],
		['SEA', 'Security, Encryption, Authorization — the crypto suite: keypairs, signing, encryption, proof-of-work.'],
		['DAM', 'Daisy-chain Ad-hoc Mesh — the message routing layer: dedup, batching, ACK tracing, heartbeats.'],
		['AXE', 'Aggregate eXchange Equation — routing optimizations: subscription-based delivery, mob rebalancing.'],
		['RAD', 'Radix Storage Engine (a.k.a. radisk) — chunked radix-tree persistence; writes the radata directory.'],
		['LEX', 'Lexical match syntax for queries: exact, prefix, and range filters over keys.'],
		['Peer', 'Any connected gunmetal or GUN.js instance — browser tab, relay, or native process.'],
		['Certificate', 'Not a TLS certificate — a signed grant from a data owner saying who may write where in their namespace.'],
		['radata', 'The on-disk directory a relay’s RAD storage writes chunks into.']
	];
</script>

<svelte:head><title>Glossary — Gunmetal</title></svelte:head>

<h1 class="text-3xl font-bold tracking-tight">Glossary</h1>
<p class="mt-3 text-muted-foreground">Every acronym and term of art used across these docs.</p>

<dl class="mt-8 space-y-4" data-testid="glossary-terms">
	{#each terms as [term, definition] (term)}
		<div>
			<dt class="font-semibold">{term}</dt>
			<dd class="text-sm text-muted-foreground">{definition}</dd>
		</div>
	{/each}
</dl>
