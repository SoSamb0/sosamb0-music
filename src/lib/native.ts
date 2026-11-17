// Glue for things that live outside the window: media keys, the tray icon, global hotkeys.

import { player } from './player.svelte';
import { settings } from './settings.svelte';
import { app } from './app.svelte';
import { isMobile } from './platform';
import { inTauri, listen } from './tauri';
import { closeWindow, showWindow } from './window';

export const GLOBAL_HOTKEYS: { keys: string; label: string; action: string }[] = [
	{ keys: 'Ctrl+Alt+Space', label: 'Играть / пауза', action: 'toggle' },
	{ keys: 'Ctrl+Alt+Right', label: 'Следующий трек', action: 'next' },
	{ keys: 'Ctrl+Alt+Left', label: 'Предыдущий трек', action: 'previous' },
	{ keys: 'Ctrl+Alt+Up', label: 'Громче', action: 'volumeUp' },
	{ keys: 'Ctrl+Alt+Down', label: 'Тише', action: 'volumeDown' }
];

let quitting = false;

/** True once the user chose "Quit" from the tray, so close-to-tray must not intercept it. */
export const isQuitting = () => quitting;

export function runAction(action: string) {
	switch (action) {
		case 'toggle':
			return player.toggle();
		case 'play':
			return player.playing ? undefined : player.toggle();
		case 'pause':
			return player.pause();
		case 'stop':
			return player.stop();
		case 'next':
			return player.next();
		case 'previous':
			return player.previous();
		case 'volumeUp':
			return player.setVolume(settings.data.volume + 0.05);
		case 'volumeDown':
			return player.setVolume(settings.data.volume - 0.05);
		case 'show':
			return showWindow();
		case 'quit':
			quitting = true;
			return closeWindow();
	}
	if (action.startsWith('seek:')) player.seek(Number(action.slice(5)));
}

export async function initNative() {
	if (!inTauri) return;
	await listen<string>('media-key', runAction);
	await listen<string>('tray', runAction);
	if (isMobile) return initBackButton();
	await listen<typeof settings.discordStatus>('discord', (status) => (settings.discordStatus = status));
	await applyHotkeys();
}

/** Android's back gesture closes whatever is on top, then walks the history, then leaves the app. */
async function initBackButton() {
	const { onBackButtonPress, exit } = await import('@tauri-apps/api/app');
	await onBackButtonPress(() => {
		if (!app.closeTop()) exit(0).catch(() => {});
	});
}

/** Register or release the global hotkeys according to the setting. */
export async function applyHotkeys(): Promise<boolean> {
	if (!inTauri || isMobile) return true;
	const { register, unregisterAll } = await import('@tauri-apps/plugin-global-shortcut');
	await unregisterAll().catch(() => {});
	if (!settings.data.hotkeys) return true;
	let ok = true;
	for (const hotkey of GLOBAL_HOTKEYS) {
		try {
			await register(hotkey.keys, (event) => {
				if (event.state === 'Pressed') runAction(hotkey.action);
			});
		} catch {
			// Another program already owns this combination.
			ok = false;
		}
	}
	return ok;
}
