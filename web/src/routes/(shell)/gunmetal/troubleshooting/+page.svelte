<script lang="ts">
	const symptoms = [
		{
			symptom: "My put isn't syncing",
			checklist: [
				'Is the relay running and reachable? Check its /health endpoint.',
				'Open the wire inspector demo and compare frames against your client.',
				'Writes from inside .on() callbacks deadlock — move writes out of subscription callbacks.'
			],
			link: '/gunmetal/demos/wire-inspector'
		},
		{
			symptom: 'Auth fails after reload',
			checklist: [
				'Persist and restore the keypair (authPair), not the alias/password.',
				'Check that storage (localStorage/IndexedDB) is enabled in your context.'
			],
			link: '/gunmetal/demos/login'
		},
		{
			symptom: 'Data missing after relay restart',
			checklist: [
				'Was the relay started with a storage directory (--file radata)?',
				'In-memory relays (tests) forget everything on restart by design.'
			],
			link: '/gunmetal/demos/offline-first'
		},
		{
			symptom: 'Relay connection refused',
			checklist: [
				'Default port is 8765 and path is /gun — the full URL is ws://host:8765/gun.',
				'A relay over capacity sheds new peers (mob) — check --mob and the dam:mob redirect.'
			],
			link: '/gunmetal/reference/relay'
		}
	];
</script>

<svelte:head><title>Troubleshooting — Gunmetal</title></svelte:head>

<h1 class="text-3xl font-bold tracking-tight">Troubleshooting</h1>
<p class="mt-3 text-muted-foreground">
	Symptom-first debugging. The
	<a class="underline" href="/gunmetal/demos/wire-inspector">wire inspector</a> is the universal
	first step: it shows you exactly what's crossing the wire.
</p>

<div class="mt-8 space-y-6" data-testid="symptom-list">
	{#each symptoms as item (item.symptom)}
		<section class="rounded-lg border p-4">
			<h2 class="font-semibold">{item.symptom}</h2>
			<ul class="mt-2 list-disc pl-6 text-sm text-muted-foreground">
				{#each item.checklist as step, i (i)}
					<li>{step}</li>
				{/each}
			</ul>
			<a class="mt-2 inline-block text-sm underline" href={item.link}>Related page →</a>
		</section>
	{/each}
</div>
