// Window chrome helpers. All of them are no-ops outside Tauri.

import { getCurrentWebview } from '@tauri-apps/api/webview';
import {
	getCurrentWindow,
	LogicalSize,
	monitorFromPoint,
	PhysicalPosition,
	PhysicalSize,
	primaryMonitor,
	type Monitor,
	type Window
} from '@tauri-apps/api/window';
import { inTauri } from './tauri';

const win = (): Window | null => (inTauri ? getCurrentWindow() : null);

export const minimize = () => win()?.minimize();
export const toggleMaximize = () => win()?.toggleMaximize();
export const closeWindow = () => win()?.close();
export const hideWindow = () => win()?.hide();

export async function showWindow() {
	const w = win();
	if (!w) return;
	await w.unminimize();
	await w.show();
	await w.setFocus();
}

/** Interface scale (browser-style zoom, so all layout maths stays correct). */
export async function setZoom(scale: number) {
	if (!inTauri) return;
	await getCurrentWebview()
		.setZoom(scale)
		.catch(() => {});
}

const MINI_POS_KEY = 'lumen.miniPosition';
const MINI = { width: 420, height: 132 };
const FULL_MIN = { width: 940, height: 600 };

/** Where the full window was before shrinking to the mini player (physical pixels). */
let saved: { x: number; y: number; width: number; height: number; maximized: boolean } | null = null;

/** The monitor under the centre of the given rectangle, or the primary one if it is off every screen. */
async function monitorFor(x: number, y: number, width: number, height: number): Promise<Monitor | null> {
	return (await monitorFromPoint(x + width / 2, y + height / 2).catch(() => null)) ?? (await primaryMonitor().catch(() => null));
}

/**
 * Move (and if needed shrink) the window so that all of it is on a screen, inside the area not
 * covered by the taskbar. Fixes windows that end up half outside after leaving the mini player.
 */
export async function keepOnScreen(w: Window | null = win()) {
	if (!w) return;
	const pos = await w.outerPosition();
	const size = await w.outerSize();
	const monitor = await monitorFor(pos.x, pos.y, size.width, size.height);
	if (!monitor) return;
	const area = monitor.workArea ?? { position: monitor.position, size: monitor.size };
	const ax = area.position.x;
	const ay = area.position.y;
	const aw = area.size.width;
	const ah = area.size.height;

	let width = size.width;
	let height = size.height;
	if (width > aw || height > ah) {
		width = Math.min(width, Math.round(aw * 0.92));
		height = Math.min(height, Math.round(ah * 0.92));
		await w.setSize(new PhysicalSize(width, height));
	}
	const x = Math.min(Math.max(pos.x, ax), ax + aw - width);
	const y = Math.min(Math.max(pos.y, ay), ay + ah - height);
	if (x !== pos.x || y !== pos.y) await w.setPosition(new PhysicalPosition(x, y));
}

/** Shrink to a small always-on-top strip, or bring the full window back where it was. */
export async function setMiniMode(on: boolean) {
	const w = win();
	if (!w) return;
	if (on) {
		const maximized = await w.isMaximized();
		if (maximized) await w.unmaximize();
		const pos = await w.outerPosition();
		const size = await w.outerSize();
		saved = { x: pos.x, y: pos.y, width: size.width, height: size.height, maximized };

		await w.setMinSize(new LogicalSize(360, 110));
		await w.setSize(new LogicalSize(MINI.width, MINI.height));
		// Reopen the mini player where it was last left.
		try {
			const last = JSON.parse(localStorage.getItem(MINI_POS_KEY) ?? 'null');
			if (last && Number.isFinite(last.x) && Number.isFinite(last.y)) await w.setPosition(new PhysicalPosition(last.x, last.y));
		} catch {
			// No remembered position.
		}
		await keepOnScreen(w);
		await w.setAlwaysOnTop(true);
	} else {
		const pos = await w.outerPosition();
		try {
			localStorage.setItem(MINI_POS_KEY, JSON.stringify({ x: pos.x, y: pos.y }));
		} catch {
			// Storage unavailable: the mini player opens where the window is.
		}
		await w.setAlwaysOnTop(false);
		await w.setMinSize(new LogicalSize(FULL_MIN.width, FULL_MIN.height));
		if (saved) {
			await w.setSize(new PhysicalSize(saved.width, saved.height));
			await w.setPosition(new PhysicalPosition(saved.x, saved.y));
		} else {
			await w.setSize(new LogicalSize(1280, 800));
			await w.center();
		}
		// The screen layout may have changed meanwhile (monitor unplugged, resolution).
		await keepOnScreen(w);
		if (saved?.maximized) await w.maximize();
		saved = null;
		await w.setFocus();
	}
}

/** `shouldHide` decides whether closing hides to the tray instead of quitting. */
export async function onCloseRequested(shouldHide: () => boolean) {
	const w = win();
	if (!w) return;
	await w.onCloseRequested((event) => {
		if (shouldHide()) {
			event.preventDefault();
			w.hide();
		}
	});
}
