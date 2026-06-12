<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		Chapter 5 left an open wound: anyone can write anywhere. Closing it without a central
		account database is what <strong>SEA</strong> (Security, Encryption, Authorization —
		gunmetal's crypto suite) is for. The core move: <em>identity is a keypair, not a row in
		someone's users table</em>.
	</p>
	<pre><code>{`const user = new WasmUser(gun);
user.create('ada', 'correct horse battery');  // derives + encrypts a keypair
user.auth('ada', 'correct horse battery');    // decrypts it back
user.put('bio', JSON.stringify('mathematician'));  // writes to ~<pub>/bio`}</code></pre>
	<p>
		Signing up generates a signing keypair and an encryption keypair, encrypts the private
		halves with a key <em>derived from your password</em> (PBKDF2 work — deliberately slow),
		and stores that encrypted blob in the graph itself. "Logging in" is just decrypting your
		own keys — any peer can serve the blob; none can open it. There is no password reset,
		because there is nobody to reset it: the keys are the account.
	</p>
	<p>
		An authenticated user writes into a <strong>user namespace</strong>: souls starting with
		<code>~&lt;public key&gt;</code>. Writes there are <em>signed</em>, and every peer
		verifies the signature before merging — an unsigned or forged write to
		<code>~ada's-key/bio</code> is simply rejected, by everyone, including relays. That's the
		answer to chapter 5's open wound: protected data lives in signed namespaces.
	</p>
	<p>
		Sessions restore without the password: the authenticated pair can be exported
		(<code>pairJson()</code>) and fed back (<code>authPair(...)</code>). Where you store it is
		your risk to own — the demo uses per-tab <code>sessionStorage</code>.
	</p>
	<h2>Try it</h2>
	<p>
		Sign up, log out, try a wrong password, log back in. Each session is its own independent
		identity.
	</p>
</div>
