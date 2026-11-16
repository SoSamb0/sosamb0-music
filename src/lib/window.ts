// Window chrome helpers. All of them are no-ops outside Tauri.

import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { inTauri } from './tauri';

const win = () => (inTauri ? getCurrentWindow() : null);

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

let savedSize: { width: number; height: number } | null = null;

/** Shrink to a small always-on-top strip, or restore the previous size. */
export async function setMiniMode(on: boolean) {
	const w = win();
	if (!w) return;
	if (on) {
		if (await w.isMaximized()) await w.unmaximize();
		const scale = await w.scaleFactor();
		const size = await w.innerSize();
		savedSize = { width: size.width / scale, height: size.height / scale };
		await w.setMinSize(new LogicalSize(360, 120));
		await w.setSize(new LogicalSize(420, 132));
		await w.setAlwaysOnTop(true);
	} else {
		await w.setAlwaysOnTop(false);
		await w.setSize(new LogicalSize(savedSize?.width ?? 1280, savedSize?.height ?? 800));
		await w.setMinSize(new LogicalSize(940, 600));
	}
}

/** Called with `true` when the user asks to close; return true to keep running in the tray. */
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
