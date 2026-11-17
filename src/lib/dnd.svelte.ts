// Dragging tracks and albums around the app: onto a playlist in the sidebar, into the queue,
// or within a playlist to change its order.

import type { Song } from './types';

export const DRAG_MIME = 'application/x-sosamb0-tracks';

interface Payload {
	/** Shown on the drag image, e.g. "3 трека". */
	label: string;
	/** Resolves the tracks being dragged (albums are loaded only when dropped). */
	load: () => Promise<Song[]> | Song[];
	/** Identifies the list a reorder drag started in. */
	source?: symbol;
	/** Row positions being dragged within `source`. */
	indexes?: number[];
}

class Drag {
	payload = $state<Payload | null>(null);
}

export const drag = new Drag();

export function trackLabel(n: number): string {
	const mod10 = n % 10;
	const mod100 = n % 100;
	const word = mod10 === 1 && mod100 !== 11 ? 'трек' : mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14) ? 'трека' : 'треков';
	return `${n} ${word}`;
}

export function startDrag(e: DragEvent, payload: Payload) {
	if (!e.dataTransfer) return;
	drag.payload = payload;
	e.dataTransfer.effectAllowed = 'copyMove';
	e.dataTransfer.setData(DRAG_MIME, payload.label);
	e.dataTransfer.setData('text/plain', payload.label);

	// A compact pill instead of a screenshot of the whole row.
	const ghost = document.createElement('div');
	ghost.className = 'drag-ghost';
	ghost.textContent = payload.label;
	document.body.appendChild(ghost);
	e.dataTransfer.setDragImage(ghost, 18, 18);
	setTimeout(() => ghost.remove());
}

export function endDrag() {
	drag.payload = null;
}

/** True while something from inside the app (not a file from Explorer) is being dragged. */
export function isAppDrag(e: DragEvent): boolean {
	return !!drag.payload && !!e.dataTransfer?.types.includes(DRAG_MIME);
}

/** Move the rows at `moving` so they sit before position `target`. Returns the new order of old indexes. */
export function reorder(length: number, moving: number[], target: number): number[] {
	const set = new Set(moving);
	const rest = [...Array(length).keys()].filter((i) => !set.has(i));
	const before = [...Array(target).keys()].filter((i) => !set.has(i)).length;
	const block = [...moving].sort((a, b) => a - b);
	return [...rest.slice(0, before), ...block, ...rest.slice(before)];
}
