<script lang="ts">
	import { app } from '$lib/app.svelte';

	let value = $state('');
	let input = $state<HTMLInputElement>();

	$effect(() => {
		if (app.question) {
			value = app.question.value;
			queueMicrotask(() => input?.focus());
		}
	});
</script>

<svelte:window onkeydown={(e) => app.question?.confirm && e.key === 'Escape' && app.answer(null)} />

<div class="toasts" aria-live="polite">
	{#each app.toasts as toast (toast.id)}
		<div class="toast" class:error={toast.kind === 'error'} role={toast.kind === 'error' ? 'alert' : 'status'}>{toast.text}</div>
	{/each}
</div>

{#if app.question}
	<div class="veil" onclick={() => app.answer(null)} role="presentation"></div>
	<form
		class="dialog"
		onsubmit={(e) => {
			e.preventDefault();
			app.answer(app.question?.confirm ? 'yes' : value);
		}}
	>
		<h2>{app.question.title}</h2>
		{#if !app.question.confirm}
			<input
				class="field"
				bind:this={input}
				bind:value
				onkeydown={(e) => {
					e.stopPropagation();
					if (e.key === 'Escape') app.answer(null);
				}}
				aria-label={app.question.title}
			/>
		{/if}
		<div class="actions">
			<button class="btn" type="button" onclick={() => app.answer(null)}>Отмена</button>
			{#if app.question.confirm}
				<button class="btn primary danger" type="submit">{app.question.confirm}</button>
			{:else}
				<button class="btn primary" type="submit" disabled={!value.trim()}>Готово</button>
			{/if}
		</div>
	</form>
{/if}

<style>
	.toasts {
		position: fixed;
		left: 50%;
		bottom: 104px;
		z-index: 950;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		transform: translateX(-50%);
		pointer-events: none;
	}

	.toast {
		max-width: min(560px, 90vw);
		padding: 10px 16px;
		border-radius: 999px;
		background: rgb(34 34 42 / 0.95);
		border: 1px solid var(--line);
		box-shadow: 0 10px 30px rgb(0 0 0 / 0.5);
		font-size: 13.5px;
		font-weight: 550;
		animation: toast-in 0.25s var(--ease);
	}

	.toast.error {
		border-radius: 14px;
		border-color: rgb(255 107 107 / 0.4);
		color: #ffb4b4;
	}

	@keyframes toast-in {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
	}

	.veil {
		position: fixed;
		inset: 0;
		z-index: 920;
		background: rgb(0 0 0 / 0.5);
	}

	.dialog {
		position: fixed;
		left: 50%;
		top: 32%;
		z-index: 921;
		display: flex;
		flex-direction: column;
		gap: 16px;
		width: min(400px, 90vw);
		padding: 24px;
		border-radius: 18px;
		background: #1c1c22;
		border: 1px solid var(--line);
		box-shadow: 0 30px 80px rgb(0 0 0 / 0.6);
		transform: translateX(-50%);
	}

	.btn.danger {
		background: var(--danger);
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
