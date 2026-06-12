<script lang="ts">
	// Presence: room-scoped heartbeat souls (NOT raw mesh peer lists —
	// spec §4 note) plus a bye() registration so the RELAY writes the
	// "left" marker the moment this client disconnects.
	import { onMount } from 'svelte';
	import {
		bootGunmetal,
		installReadyPromise,
		markDegraded,
		markReady,
		parseClientParams,
		relayRunHint,
		type WasmGunInstance
	} from '$lib/gun/client';

	let { slug = 'presence' }: { slug?: string } = $props();

	const HEARTBEAT_MS = 2000;
	const ONLINE_WINDOW_MS = 8000;

	let members = $state<Record<string, number>>({});
	let departed = $state<Record<string, boolean>>({});
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let hint = $state('');
	let frameId = $state('');
	let now = $state(Date.now());

	let gun: WasmGunInstance | undefined;
	let roomSoul = '';
	let lastSeenSoul = '';

	const roster = $derived(
		Object.entries(members)
			.map(([id, at]) => ({
				id,
				online: now - at < ONLINE_WINDOW_MS && !departed[id],
				left: departed[id] === true
			}))
			.sort((a, b) => a.id.localeCompare(b.id))
	);

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		frameId = params.frameId;
		roomSoul = `demo/${slug}/${params.room}`;
		lastSeenSoul = `demo/${slug}-left/${params.room}`;

		try {
			gun = await bootGunmetal(params);

			// Node subscriptions fire per key: (value, key) — here key is
			// the session id.
			gun.onNode(roomSoul, (json: string, id: string) => {
				const at = JSON.parse(json);
				if (typeof at === 'number') {
					members = { ...members, [id]: at };
					now = Date.now();
				}
			});
			gun.onNode(lastSeenSoul, (json: string, id: string) => {
				if (JSON.parse(json) === true) {
					departed = { ...departed, [id]: true };
				}
			});
			gun.fetchSoul(roomSoul);
			gun.fetchSoul(lastSeenSoul);

			// The relay writes our departure for us when the socket drops.
			gun.registerBye(lastSeenSoul, frameId, 'true');

			const beat = () => {
				gun?.putNumber(roomSoul, frameId, Date.now());
				now = Date.now();
			};
			beat();
			setInterval(beat, HEARTBEAT_MS);

			status = 'ready';
			markReady();
		} catch (cause) {
			hint = relayRunHint(params.relay);
			status = 'degraded';
			markDegraded();
			console.error(cause);
		}
	});
</script>

<div class="flex min-h-screen flex-col bg-background p-4 text-foreground">
	{#if status === 'booting'}
		<p class="text-sm text-muted-foreground" data-testid="client-booting">Starting gunmetal…</p>
	{:else if status === 'degraded'}
		<div class="rounded-lg border border-destructive/50 p-4 text-sm" data-testid="client-degraded">
			<p class="font-medium">Relay unreachable</p>
			<pre class="mt-2 whitespace-pre-wrap text-xs text-muted-foreground">{hint}</pre>
		</div>
	{:else}
		<p class="text-xs uppercase tracking-wide text-muted-foreground">
			You are session {frameId}
		</p>
		<ul class="mt-3 space-y-2" data-testid="presence-roster">
			{#each roster as member (member.id)}
				<li
					class="flex items-center gap-2 text-sm"
					data-member={member.id}
					data-online={member.online}
					data-left={member.left}
				>
					<span
						class="inline-block size-2.5 rounded-full {member.left
							? 'bg-destructive'
							: member.online
								? 'bg-green-500'
								: 'bg-muted-foreground/40'}"
					></span>
					Session {member.id}
					{#if member.left}<span class="text-xs text-muted-foreground">(left)</span>{/if}
				</li>
			{/each}
		</ul>
	{/if}
</div>
