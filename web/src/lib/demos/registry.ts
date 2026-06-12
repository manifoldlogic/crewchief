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
import ConflictLabClient from './conflict-lab/ConflictLabClient.svelte';
import OfflineFirstClient from './offline-first/OfflineFirstClient.svelte';
import LoginClient from './login/LoginClient.svelte';
import PrivateNotesClient from './private-notes/PrivateNotesClient.svelte';
import SecretHandshakeClient from './secret-handshake/SecretHandshakeClient.svelte';
import DocPermissionsClient from './doc-permissions/DocPermissionsClient.svelte';

export const clientRegistry: Record<string, Component<{ slug?: string }>> = {
	'shared-input': SharedInputClient,
	'gunjs-interop': InteropClient,
	'graph-explorer': GraphExplorerClient,
	'chat-room': ChatRoomClient,
	'todo-list': TodoListClient,
	presence: PresenceClient,
	'profile-tree': ProfileTreeClient,
	'conflict-lab': ConflictLabClient,
	'offline-first': OfflineFirstClient,
	login: LoginClient,
	'private-notes': PrivateNotesClient,
	'secret-handshake': SecretHandshakeClient,
	'doc-permissions': DocPermissionsClient
};
