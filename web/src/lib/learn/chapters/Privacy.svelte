<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		Identity (chapter 8) controls who can <em>write</em>. It does nothing about who can
		<em>read</em> — the graph replicates to every interested peer, relays included. Privacy in
		a decentralized system means one thing: <strong>encrypt before you put</strong>.
	</p>
	<pre><code>{`const sea = new WasmSEA();
const key = sea.work('a passphrase', salt);     // slow derivation (PBKDF2)
const ct  = sea.encrypt(JSON.stringify(note), key);
gun.setValue(notesSoul, JSON.stringify(ct));    // the graph stores ciphertext`}</code></pre>
	<p>
		What peers and relays store and forward is ciphertext (AES-GCM under the hood). Anyone
		with the passphrase derives the same key — using a <em>fixed salt</em> — and decrypts;
		everyone else holds noise.
	</p>
	<p>
		Passphrases are for shared spaces. For two <em>people</em>, you don't pre-share anything:
		each side has an encryption keypair (the <code>epub</code>/<code>epriv</code> half of a
		SEA pair), and
	</p>
	<pre><code>{`sea.secret(their_epub, my_epriv)  ===  sea.secret(my_epub, their_epriv)`}</code></pre>
	<p>
		— an <strong>ECDH</strong> (elliptic-curve Diffie–Hellman) exchange: both sides compute
		the same shared key from public+own-private, while an observer holding both public keys
		computes nothing. Publish only your <code>epub</code>, derive, encrypt, send through the
		public graph.
	</p>
	<h2>Try it</h2>
	<p>
		Private notes shows what the graph <em>actually stores</em> next to what the passphrase
		unlocks. Secret handshake runs the ECDH exchange live between the two sessions — see the
		<a href="/gunmetal/demos/secret-handshake">full demo</a>.
	</p>
</div>
