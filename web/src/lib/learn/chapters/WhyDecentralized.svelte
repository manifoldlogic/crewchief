<div class="prose prose-neutral dark:prose-invert max-w-none">
	<p>
		Most web apps work like this: your browser asks a server for data, the server asks a
		database, and whatever the database says is the truth. If the server is down, the app is
		down. If the company turns it off, your data is gone.
	</p>
	<p>
		<strong>Decentralized apps</strong> flip that model: every participant keeps its own copy
		of the data, and copies synchronize with each other. There is no single machine that owns
		the truth. Gunmetal is a database built for that model — and because peers can disagree
		(two people edit the same thing while offline), a decentralized database needs rules for
		merging that never require a referee. Those rules are the heart of this course.
	</p>
	<h2>What's a graph?</h2>
	<p>
		You probably know tables (rows and columns) or documents (nested JSON). Gunmetal uses a
		<strong>graph</strong>: a soup of small flat records called <strong>nodes</strong>, where
		any node can point at any other node. A pointer from one node to another is a
		<strong>link</strong>. Tables make you decide the shape up front; documents trap data
		inside one big tree. A graph lets every piece of data be addressed, shared, and updated
		independently — which is exactly what synchronizing peers need.
	</p>
	<p>
		Every node has exactly one globally-unique id called its <strong>soul</strong> (GUN's
		term — think "the name that makes a node the same node on every device in the world").
		Two peers that both write to soul <code>people/ada</code> are, by definition, editing the
		same record.
	</p>
	<h2>Where gunmetal comes from</h2>
	<p>
		<a href="https://gun.eco" rel="external">GUN.js</a> is a popular decentralized graph
		database for JavaScript. <strong>Gunmetal</strong> is an independent implementation of the
		same protocol in Rust: it speaks GUN's wire format, merges with GUN's rules, and runs the
		same mesh — so a gunmetal peer and a GUN.js peer can sync with each other (you'll see
		that live in the final chapter). Gunmetal compiles to native code (for servers and CLIs)
		and to WebAssembly (for browsers — every demo on this site runs it).
	</p>
	<p>
		That's all the theory you need up front. Next: actually putting data into the graph.
	</p>
</div>
