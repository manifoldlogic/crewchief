/**
 * Maps demo slugs to their client components (rendered by the bare
 * /client pages inside iframes). Only imported by the client route.
 */
import type { Component } from 'svelte';
import SharedInputClient from './shared-input/SharedInputClient.svelte';
import InteropClient from './gunjs-interop/InteropClient.svelte';
import GraphExplorerClient from './graph-explorer/GraphExplorerClient.svelte';
import ChatRoomClient from './chat-room/ChatRoomClient.svelte';
import TodoListClient from './todo-list/TodoListClient.svelte';
import PresenceClient from './presence/PresenceClient.svelte';
import ProfileTreeClient from './profile-tree/ProfileTreeClient.svelte';

export const clientRegistry: Record<string, Component<{ slug?: string }>> = {
	'shared-input': SharedInputClient,
	'gunjs-interop': InteropClient,
	'graph-explorer': GraphExplorerClient,
	'chat-room': ChatRoomClient,
	'todo-list': TodoListClient,
	presence: PresenceClient,
	'profile-tree': ProfileTreeClient
};
