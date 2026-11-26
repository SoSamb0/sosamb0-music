// Headphone correction profiles from the AutoEQ project (github.com/jaakkopasanen/AutoEq):
// the index of every measured model, searched here, and each model's ParametricEQ.txt.

const BASE = 'https://raw.githubusercontent.com/jaakkopasanen/AutoEq/master/results/';

export interface AutoEqEntry {
	name: string;
	/** Folder under results/, URL-encoded as in the index. */
	path: string;
	/** Who measured it (oratory1990, crinacle, …). */
	source: string;
	form: 'in-ear' | 'over-ear' | 'earbud' | 'other';
	/** Lower-case name for matching. */
	key: string;
}

// Measurements AutoEQ itself recommends first when a model was measured by several people.
const SOURCE_RANK = ['oratory1990', 'crinacle', 'Rtings', 'Innerfidelity', 'Headphone.com Legacy'];

let index: Promise<AutoEqEntry[]> | null = null;

/** The list of every profile. Downloaded once per session (about 0.9 MB). */
export function loadIndex(): Promise<AutoEqEntry[]> {
	index ??= fetch(`${BASE}INDEX.md`)
		.then((r) => {
			if (!r.ok) throw new Error(`AutoEQ: ${r.status}`);
			return r.text();
		})
		.then(parseIndex)
		.catch((e) => {
			index = null; // try again next time
			throw e;
		});
	return index;
}

export function parseIndex(text: string): AutoEqEntry[] {
	const out: AutoEqEntry[] = [];
	for (const line of text.split('\n')) {
		const m = line.match(/^- \[(.*)\]\(\.\/(.*)\) by (.*?)(?: on .*)?\s*$/);
		if (!m) continue;
		const folder = decodeURIComponent(m[2]).toLowerCase();
		const form = folder.includes('in-ear') ? 'in-ear' : folder.includes('over-ear') ? 'over-ear' : folder.includes('earbud') ? 'earbud' : 'other';
		out.push({ name: m[1], path: m[2], source: m[3], form, key: m[1].toLowerCase() });
	}
	return out;
}

/** Models whose name contains every word of `query`, best matches first. */
export function searchIndex(entries: AutoEqEntry[], query: string, limit = 40): AutoEqEntry[] {
	const words = query.toLowerCase().split(/\s+/).filter(Boolean);
	if (!words.length) return [];
	const rank = (s: string) => {
		const i = SOURCE_RANK.indexOf(s);
		return i < 0 ? SOURCE_RANK.length : i;
	};
	const first = words[0];
	return entries
		.filter((e) => words.every((w) => e.key.includes(w)))
		.map((e) => ({ e, score: (e.key.startsWith(first) ? 0 : e.key.includes(` ${first}`) ? 1 : 2) * 1000 + e.key.length + rank(e.source) * 0.1 }))
		.sort((a, b) => a.score - b.score)
		.slice(0, limit)
		.map((x) => x.e);
}

/** The model's ParametricEQ.txt (Equalizer APO format). */
export async function fetchProfile(entry: AutoEqEntry): Promise<string> {
	const folder = decodeURIComponent(entry.path.split('/').pop() ?? entry.name);
	const r = await fetch(`${BASE}${entry.path}/${encodeURIComponent(`${folder} ParametricEQ.txt`)}`);
	if (!r.ok) throw new Error(r.status === 404 ? 'Для этой модели нет параметрического профиля' : `AutoEQ: ${r.status}`);
	return r.text();
}

export const FORMS: Record<AutoEqEntry['form'], string> = {
	'in-ear': 'внутриканальные',
	'over-ear': 'полноразмерные',
	earbud: 'вкладыши',
	other: ''
};
