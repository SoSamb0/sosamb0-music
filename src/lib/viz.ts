// Latest analysis frame from the audio engine. Deliberately not reactive: it changes ~30 times
// a second and is read only by the canvas drawing loop.

import { invoke } from './tauri';

export interface VizFrame {
	bands: number[];
	wave: number[];
	level: [number, number];
}

export const viz = {
	frame: null as VizFrame | null,
	at: 0
};

export function setVizFrame(frame: VizFrame) {
	viz.frame = frame;
	viz.at = performance.now();
}

let users = 0;

/** Ask the engine for frames while at least one visualiser is mounted. Returns the release function. */
export function useVisualizer(): () => void {
	if (users++ === 0) invoke('player_set_visualizer', { on: true });
	let released = false;
	return () => {
		if (released) return;
		released = true;
		if (--users === 0) {
			invoke('player_set_visualizer', { on: false });
			viz.frame = null;
		}
	};
}
