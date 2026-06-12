/**
 * Typed postMessage protocol between demo pages and their client iframes
 * (spec §5.5). Parent → child controls relay connectivity (conflict-lab,
 * offline-first); child → parent reports status changes.
 */

export type ParentToChild =
	| { type: 'gm:disconnect' }
	| { type: 'gm:reconnect' };

export type ChildToParent = {
	type: 'gm:status';
	frameId: string;
	status: 'ready' | 'degraded' | 'connected' | 'disconnected';
};

export function sendToFrame(frame: HTMLIFrameElement, msg: ParentToChild): void {
	frame.contentWindow?.postMessage(msg, window.location.origin);
}

export function sendToParent(msg: ChildToParent): void {
	window.parent?.postMessage(msg, window.location.origin);
}

export function onParentMessage(handler: (msg: ParentToChild) => void): () => void {
	const listener = (event: MessageEvent) => {
		if (event.origin !== window.location.origin) return;
		const data = event.data;
		if (data && typeof data === 'object' && typeof data.type === 'string' && data.type.startsWith('gm:')) {
			handler(data as ParentToChild);
		}
	};
	window.addEventListener('message', listener);
	return () => window.removeEventListener('message', listener);
}
