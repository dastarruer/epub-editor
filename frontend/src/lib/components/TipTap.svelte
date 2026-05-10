<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Editor } from '@tiptap/core';
	import { Document } from '@tiptap/extension-document';
	import { Paragraph } from '@tiptap/extension-paragraph';
	import { Text } from '@tiptap/extension-text';
	import { Heading } from '@tiptap/extension-heading';
	import { Bold } from '@tiptap/extension-bold';
	import { Italic } from '@tiptap/extension-italic';
	import { History } from '@tiptap/extension-history';
	import BubbleMenu from '@tiptap/extension-bubble-menu';

	let bubbleMenu: HTMLElement | undefined = $state();
	let element: HTMLElement | undefined = $state();
	let editorState: { editor: Editor | null } = $state({ editor: null });

	let { content } = $props();

	onMount(() => {
		editorState.editor = new Editor({
			element: element,
			extensions: [
				Document,
				Paragraph,
				Text,
				Heading.configure({ levels: [1, 2, 3] }),
				Bold,
				Italic,
				History,
				BubbleMenu.configure({ element: bubbleMenu })
			],
			content: content,
			onTransaction: ({ editor }) => {
				// Update the state signal to force a re-render
				editorState = { editor };
			}
		});
	});
	onDestroy(() => {
		editorState.editor?.destroy();
	});
</script>

<div style="position: relative" class="app">
	{#if editorState.editor}
		<div class="fixed-menu">
			<button
				onclick={() => editorState.editor?.chain().focus().toggleHeading({ level: 1 }).run()}
				class:active={editorState.editor?.isActive('heading', { level: 1 })}
			>
				H1
			</button>
			<button
				onclick={() => editorState.editor?.chain().focus().toggleHeading({ level: 2 }).run()}
				class:active={editorState.editor.isActive('heading', { level: 2 })}
			>
				H2
			</button>
			<button
				onclick={() => editorState.editor?.chain().focus().setParagraph().run()}
				class:active={editorState.editor.isActive('paragraph')}
			>
				P
			</button>
		</div>
	{/if}

	<div bind:this={element}></div>
</div>

<style>
	button.active {
		background: black;
		color: white;
	}
</style>
