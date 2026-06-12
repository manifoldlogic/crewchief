<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		A gunmetal graph is made of <strong>nodes</strong>: flat records of key→value pairs. Every
		node lives at a <strong>soul</strong> — its globally-unique id (defined in chapter 0; also
		in the <a href="/gunmetal/glossary">glossary</a>). Writing is <code>put</code>, reading is
		<code>get</code>:
	</p>
	<pre><code>{`gun.putText('people/ada', 'name', 'Ada');   // write
gun.get('people/ada', 'name');              // → "Ada"`}</code></pre>
	<p>
		There is no "create table" and no schema: putting to a soul that doesn't exist yet creates
		the node. Values are text, numbers, booleans, null — or a <strong>link</strong> to another
		node:
	</p>
	<pre><code>{`gun.putLink('people/ada', 'employer', 'orgs/analytical-engines');`}</code></pre>
	<p>
		A link stores the target's soul (on the wire it looks like
		<code>{'{"#": "orgs/analytical-engines"}'}</code>). Links are how flat little nodes compose
		into bigger structures — profiles, lists, whole documents — without ever nesting data
		inside one record. That separation matters for sync: each node updates and travels
		independently.
	</p>
	<h2>Try it</h2>
	<p>
		The explorer below is a real gunmetal instance running in your browser (no network — that
		comes in chapter 5). Put some values, watch the node inspector update, and use "Link from
		index" to see what a link looks like.
	</p>
</div>
