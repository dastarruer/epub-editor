<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';

	interface Metadata {
		title: string;
		year: number;
		creators: string[];
	}

	let metadata = invoke<Metadata>('read_epub_metadata');
	let content = invoke<string>('get_epub_content');
</script>

<h1>EPUB Metadata</h1>

{#await metadata}
	<p>Reading EPUB and fetching metadata...</p>
{:then metadata}
	<p><strong>Title:</strong> {metadata.title}</p>
	<p><strong>Year:</strong> {metadata.year}</p>
	<p><strong>Creators:</strong> {metadata.creators.join(', ')}</p>
{:catch error}
	<p>Error: {error}</p>
{/await}

{#await content}
	<p>Loading content...</p>
{:then html}
	<div>{@html `<base href="epub://localhost/OEBPS/">` + html}</div>
{:catch error}
	<p>Error loading content: {error}</p>
{/await}
