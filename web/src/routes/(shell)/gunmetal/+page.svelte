<script lang="ts">
	import { capabilities } from '$lib/catalog';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
</script>

<svelte:head><title>Gunmetal — crewchief docs</title></svelte:head>

<!-- §3.4 block 1: hero -->
<section class="py-4">
	<h1 class="text-4xl font-bold tracking-tight">Gunmetal</h1>
	<p class="mt-3 max-w-2xl text-lg text-muted-foreground">
		A Rust + WASM graph database that speaks the GUN.js wire protocol — CRDT conflict
		resolution, end-to-end encryption, and peer-to-peer sync.
	</p>
	<div class="mt-6 flex gap-3">
		<Button href="/gunmetal/learn" data-testid="cta-learn">New to this? Start with the basics →</Button>
		<Button href="/gunmetal/quickstart" variant="outline" data-testid="cta-quickstart">Get started</Button>
	</div>
</section>

<!-- §3.4 block 2: flagship demo (live embed lands in P3) -->
<section class="mt-10" data-testid="flagship-embed">
	<Card.Root>
		<Card.Header>
			<Card.Title>Watch gunmetal join a real GUN.js mesh</Card.Title>
			<Card.Description>
				The left client is GUN.js. The right is gunmetal. Same mesh. (Live demo coming online
				with the demo catalog —
				<a class="underline" href="/gunmetal/demos/gunjs-interop">see the demo page</a>.)
			</Card.Description>
		</Card.Header>
	</Card.Root>
</section>

<!-- §3.4 block 3: capability grid -->
<section class="mt-10">
	<h2 class="text-2xl font-semibold tracking-tight">Capabilities</h2>
	<div class="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-3" data-testid="capability-grid">
		{#each capabilities as cap (cap.id)}
			<a href={'/gunmetal/demos/' + cap.demo} class="group" data-capability={cap.id}>
				<Card.Root class="h-full transition-colors group-hover:border-foreground/30">
					<Card.Header>
						<Card.Title class="text-base">{cap.title}</Card.Title>
						<Card.Description>
							{cap.claim}
							{#if cap.badge}
								<span class="mt-1 block text-xs italic">{cap.badge}</span>
							{/if}
						</Card.Description>
					</Card.Header>
				</Card.Root>
			</a>
		{/each}
	</div>
</section>

<!-- §3.4 block 4: vs GUN.js -->
<section class="mt-10" data-testid="vs-gunjs">
	<h2 class="text-2xl font-semibold tracking-tight">Gunmetal vs GUN.js</h2>
	<div class="mt-4 grid gap-4 md:grid-cols-3">
		<Card.Root>
			<Card.Header>
				<Card.Title class="text-base">Same</Card.Title>
				<Card.Description>
					Wire protocol (<a class="underline" href="/gunmetal/demos/gunjs-interop">live proof</a>),
					HAM merge semantics, SEA API surface, RAD JSON storage format.
				</Card.Description>
			</Card.Header>
		</Card.Root>
		<Card.Root>
			<Card.Header>
				<Card.Title class="text-base">Missing</Card.Title>
				<Card.Description>WebRTC transport, UDP multicast, NTS key rotation.</Card.Description>
			</Card.Header>
		</Card.Root>
		<Card.Root>
			<Card.Header>
				<Card.Title class="text-base">Why Rust</Card.Title>
				<Card.Description>
					A single static relay binary with no Node runtime, typed embedding in Rust apps, and
					one codebase compiled to native and wasm.
				</Card.Description>
			</Card.Header>
		</Card.Root>
	</div>
</section>

<!-- §3.4 block 5: status & limits -->
<section class="mt-10" data-testid="status-limits">
	<h2 class="text-2xl font-semibold tracking-tight">Status &amp; limits</h2>
	<ul class="mt-4 list-disc space-y-1 pl-6 text-sm text-muted-foreground">
		<li>Pre-1.0 and brand new — APIs may still change.</li>
		<li>Browsers sync through a relay only; there is no WebRTC transport.</li>
		<li>No UDP multicast LAN discovery.</li>
		<li>GUN.js storage-directory compatibility is best-effort (JSON-mode chunks).</li>
		<li>Pure-Rust crypto — no WebCrypto path.</li>
	</ul>
</section>

<!-- §3.4 block 6: quick start teaser -->
<section class="mt-10 mb-4" data-testid="quickstart-teaser">
	<h2 class="text-2xl font-semibold tracking-tight">Quick start</h2>
	<p class="mt-2 text-sm text-muted-foreground">
		Use gunmetal as a Rust crate or from the browser via wasm — both paths are covered in the
		<a class="underline" href="/gunmetal/quickstart">quickstart</a>.
	</p>
</section>
