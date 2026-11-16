import { app } from './app.svelte';

/**
 * Load data for a view. `fetcher` re-runs whenever the reactive values it reads change
 * (a route id, a sort order), and a slow earlier request can never overwrite a newer one.
 */
export function loader<T>(fetcher: () => Promise<T>) {
	let value = $state<T>();
	let loading = $state(true);
	let failed = $state(false);
	let version = $state(0);

	$effect(() => {
		version;
		let stale = false;
		loading = true;
		failed = false;
		fetcher()
			.then((v) => {
				if (stale) return;
				value = v;
				loading = false;
			})
			.catch((e) => {
				if (stale) return;
				failed = true;
				loading = false;
				app.toast(e instanceof Error ? e.message : String(e), 'error');
			});
		return () => {
			stale = true;
		};
	});

	return {
		get value() {
			return value;
		},
		get loading() {
			return loading;
		},
		get failed() {
			return failed;
		},
		reload() {
			version++;
		}
	};
}
