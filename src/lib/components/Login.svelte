<script lang="ts">
	import { app } from '$lib/app.svelte';
	import { inTauri } from '$lib/tauri';
	import { closeWindow, minimize } from '$lib/window';
	import Icon from './Icon.svelte';

	let url = $state('');
	let username = $state('');
	let password = $state('');
	let error = $state('');

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		try {
			await app.login(url, username, password);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}
</script>

<div class="screen" data-tauri-drag-region>
	{#if inTauri}
		<div class="chrome">
			<button class="icon-btn" onclick={minimize} aria-label="Свернуть"><Icon name="min" size={15} /></button>
			<button class="icon-btn" onclick={closeWindow} aria-label="Закрыть"><Icon name="x" size={16} /></button>
		</div>
	{/if}

	<form class="card" onsubmit={submit}>
		<span class="logo" aria-hidden="true"></span>
		<h1>Lumen</h1>
		<p class="muted lead">Подключитесь к своему серверу Navidrome</p>

		<label>
			<span>Адрес сервера</span>
			<input class="field" bind:value={url} placeholder="http://192.168.1.10:4533" required autocomplete="url" spellcheck="false" />
		</label>
		<label>
			<span>Логин</span>
			<input class="field" bind:value={username} required autocomplete="username" spellcheck="false" />
		</label>
		<label>
			<span>Пароль</span>
			<input class="field" type="password" bind:value={password} required autocomplete="current-password" />
		</label>

		{#if error}<p class="error" role="alert">{error}</p>{/if}

		<button class="btn primary" type="submit" disabled={app.connecting}>
			{#if app.connecting}<span class="spinner small"></span>{/if}
			Подключиться
		</button>
		<button class="btn ghost" type="button" onclick={() => app.demo()}>Посмотреть на демо-библиотеке</button>
		<p class="note faint">Пароль не сохраняется: на диске остаётся только производный от него токен.</p>
	</form>
</div>

<style>
	.screen {
		position: relative;
		display: grid;
		place-items: center;
		height: 100vh;
		background:
			radial-gradient(60% 50% at 30% 20%, rgb(var(--accent-rgb) / 0.22), transparent),
			radial-gradient(50% 50% at 80% 90%, rgb(90 160 255 / 0.14), transparent), var(--bg);
	}

	.chrome {
		position: absolute;
		top: 8px;
		right: 10px;
		display: flex;
		gap: 2px;
	}

	.card {
		display: flex;
		flex-direction: column;
		gap: 14px;
		width: min(400px, calc(100vw - 40px));
		padding: 34px 32px 26px;
		border-radius: 22px;
		background: rgb(22 22 28 / 0.78);
		backdrop-filter: blur(30px);
		border: 1px solid var(--line);
		box-shadow: 0 30px 90px rgb(0 0 0 / 0.55);
		animation: page-in 0.4s var(--ease);
	}

	.logo {
		width: 44px;
		height: 44px;
		border-radius: 50%;
		background: conic-gradient(from 210deg, var(--accent), rgb(var(--accent-rgb) / 0.2), var(--accent));
		box-shadow: 0 0 34px rgb(var(--accent-rgb) / 0.6);
	}

	.lead {
		margin: -8px 0 8px;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 6px;
		font-size: 13px;
		font-weight: 550;
		color: var(--muted);
	}

	.error {
		padding: 10px 12px;
		border-radius: var(--radius-sm);
		background: rgb(255 107 107 / 0.12);
		color: var(--danger);
		font-size: 13px;
	}

	.btn {
		height: 42px;
		margin-top: 4px;
	}

	.btn.ghost {
		margin-top: -4px;
		background: none;
		color: var(--muted);
		font-weight: 500;
	}

	.btn.ghost:hover {
		color: var(--text);
		background: var(--surface);
	}

	.spinner.small {
		width: 16px;
		height: 16px;
		border-color: rgb(0 0 0 / 0.2);
		border-top-color: #0b0b10;
	}

	.note {
		font-size: 12px;
		text-align: center;
	}
</style>
