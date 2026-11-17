// Reordering a list with a finger. HTML5 drag and drop does not work on touch screens, so rows
// carry a handle: press it and drag. Rows are found by their `data-i` attribute (their index).

/** Distance from the scroller's edge (px) where the list starts to scroll by itself. */
const EDGE = 64;

function scrollParent(el: HTMLElement): HTMLElement {
	for (let p = el.parentElement; p; p = p.parentElement) {
		const style = getComputedStyle(p).overflowY;
		if ((style === 'auto' || style === 'scroll') && p.scrollHeight > p.clientHeight) return p;
	}
	return document.scrollingElement as HTMLElement;
}

/**
 * Start dragging row `from` from a pointerdown on its handle. `onGap` reports where the row would
 * land, as the index it would be inserted before (0..length), or null; `onDrop` gets the final gap
 * only when the order actually changes.
 */
export function touchReorder(
	e: PointerEvent,
	from: number,
	onGap: (gap: number | null) => void,
	onDrop: (from: number, gap: number) => void
) {
	const handle = e.currentTarget as HTMLElement;
	const row = handle.closest<HTMLElement>('[data-i]');
	const list = row?.parentElement;
	if (!row || !list) return;
	e.preventDefault();
	e.stopPropagation();
	handle.setPointerCapture(e.pointerId);
	const scroller = scrollParent(row);
	const startY = e.clientY;
	const startScroll = scroller.scrollTop;
	let y = e.clientY;
	let gap: number | null = null;
	let running = true;
	row.classList.add('lifted');
	navigator.vibrate?.(12);

	const rows = () => [...list.querySelectorAll<HTMLElement>(':scope > [data-i]')];

	const frame = () => {
		if (!running) return;
		// Scroll when the finger is near the top or bottom edge.
		const box = scroller === document.scrollingElement ? { top: 0, bottom: innerHeight } : scroller.getBoundingClientRect();
		if (y < box.top + EDGE) scroller.scrollTop -= Math.ceil((box.top + EDGE - y) / 6);
		else if (y > box.bottom - EDGE) scroller.scrollTop += Math.ceil((y - box.bottom + EDGE) / 6);

		row.style.translate = `0 ${y - startY + scroller.scrollTop - startScroll}px`;

		const all = rows();
		let g = all.length;
		for (const r of all) {
			if (r === row) continue;
			const b = r.getBoundingClientRect();
			if (y < b.top + b.height / 2) {
				g = Number(r.dataset.i);
				break;
			}
		}
		// Dropping right before or after itself changes nothing.
		const next = g === from || g === from + 1 ? null : g;
		if (next !== gap) {
			gap = next;
			onGap(gap);
		}
		requestAnimationFrame(frame);
	};
	requestAnimationFrame(frame);

	const move = (ev: PointerEvent) => {
		y = ev.clientY;
	};
	const finish = (drop: boolean) => {
		running = false;
		row.classList.remove('lifted');
		row.style.translate = '';
		handle.removeEventListener('pointermove', move);
		handle.removeEventListener('pointerup', up);
		handle.removeEventListener('pointercancel', cancel);
		onGap(null);
		if (drop && gap !== null) onDrop(from, gap);
	};
	const up = () => finish(true);
	const cancel = () => finish(false);
	handle.addEventListener('pointermove', move);
	handle.addEventListener('pointerup', up);
	handle.addEventListener('pointercancel', cancel);
}

/** Final index of a single moved row, from the gap it was dropped into. */
export const gapToIndex = (from: number, gap: number) => (gap > from ? gap - 1 : gap);
