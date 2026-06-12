<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		<em>Not TLS certificates</em> — a gunmetal <strong>certificate</strong> is a signed note
		from a data owner saying who may write where in their space. It's how you share write
		access without sharing your keys, and without a server to keep an ACL.
	</p>
	<pre><code>{`// Owner: "this pub key may write to shared-doc", signed by me.
const grant = cert.create(guestPub, 'shared-doc', undefined, myPub, myPriv);

// Guest: attach the cert to a signed entry.
const sig = sea.sign(JSON.stringify(text), myPriv, myPub);
publish({ by: myPub, sig, cert: grant });

// EVERY reader checks, independently:
cert.grantsAccess(grant, entry.by, 'shared-doc')  // scope + expiry + owner's signature
sea.verify(entry.sig, entry.by)                   // the writer really wrote this`}</code></pre>
	<p>
		The shape to internalize: <strong>enforcement happens at read time, by every reader, with
		math</strong>. The owner never has to be online; no relay has to cooperate; revocation is
		a tombstone the owner publishes; expiry is a timestamp inside the signed grant. A write
		without a valid certificate isn't blocked from existing — it's simply <em>not believed</em>
		by anyone who checks.
	</p>
	<p>
		Grants can target one key or anyone (<code>*</code>), one path, a prefix
		(<code>docs/*</code>), or everything — the
		<a href="/gunmetal/reference/cert">cert reference</a> has the matrix and the revocation
		details.
	</p>
	<h2>Try it</h2>
	<p>
		The guest writes before being granted — watch the owner's session reject it. Grant, write
		again, accepted. Both verdicts were computed locally from signatures alone.
	</p>
</div>
