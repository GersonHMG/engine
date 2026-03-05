<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { appWindow } from '@tauri-apps/api/window';
	import { Card, CardHeader, CardTitle, CardContent } from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Button } from '$lib/components/ui/button/index.js';

	interface Param {
		id: string;
		label: string;
		defaultVal: number;
		step: number;
		controller: string;
	}

	const allParams: Param[] = [
		{ id: 'ctrl-lat-kp', label: 'Lateral KP', defaultVal: 3.0, step: 0.1, controller: 'lookahead' },
		{ id: 'ctrl-lat-ki', label: 'Lateral KI', defaultVal: 0.1, step: 0.1, controller: 'lookahead' },
		{ id: 'ctrl-lat-kd', label: 'Lateral KD', defaultVal: 0.5, step: 0.1, controller: 'lookahead' },
		{ id: 'ctrl-speed-kp', label: 'Speed KP', defaultVal: 2.0, step: 0.1, controller: 'lookahead' },
		{ id: 'ctrl-head-kp', label: 'Heading KP', defaultVal: 4.0, step: 0.1, controller: 'lookahead' },
		{ id: 'ctrl-target-speed', label: 'Target Speed', defaultVal: 1.0, step: 0.1, controller: 'lookahead' },
		{ id: 'ctrl-lookahead', label: 'Lookahead Dist', defaultVal: 0.25, step: 0.05, controller: 'lookahead' },
		{ id: 'ctrl-bb-amax', label: 'BangBang Max Accel', defaultVal: 2.5, step: 0.1, controller: 'bangbang' },
		{ id: 'ctrl-bb-vmax', label: 'BangBang Max Vel', defaultVal: 5.0, step: 0.1, controller: 'bangbang' },
		{ id: 'ctrl-pid-kp', label: 'PID KP', defaultVal: 2.0, step: 0.1, controller: 'pid' },
		{ id: 'ctrl-pid-maxv', label: 'PID Max Vel', defaultVal: 1.5, step: 0.1, controller: 'pid' }
	];

	let controller = $state('lookahead');
	let values = $state<Record<string, number>>({});
	let visibleParams = $derived(allParams.filter((p) => p.controller === controller));

	const titles: Record<string, string> = {
		pid: 'PID Settings',
		lookahead: 'LookAhead PID Settings',
		bangbang: 'BangBangTrajectories Settings'
	};

	let title = $derived(titles[controller] || 'Controller Settings');

	onMount(() => {
		// Read controller from URL param
		const urlParams = new URLSearchParams(window.location.search);
		const ctrl = urlParams.get('controller');
		if (ctrl) controller = ctrl;

		// Load values from localStorage
		const loaded: Record<string, number> = {};
		for (const p of allParams) {
			const stored = localStorage.getItem(p.id);
			loaded[p.id] = stored !== null ? parseFloat(stored) : p.defaultVal;
		}
		values = loaded;
	});

	function save() {
		for (const p of allParams) {
			if (values[p.id] !== undefined) {
				localStorage.setItem(p.id, String(values[p.id]));
			}
		}
		appWindow.close();
	}

	function cancel() {
		appWindow.close();
	}
</script>

<div class="flex min-h-screen flex-col bg-background p-5 text-foreground">
	<h3 class="mb-5 text-center text-lg font-semibold">{title}</h3>

	<Card>
		<CardContent class="space-y-3 pt-4">
			{#each visibleParams as param (param.id)}
				<div class="flex items-center justify-between">
					<Label>{param.label}</Label>
					<Input
						type="number"
						step={param.step}
						bind:value={values[param.id]}
						class="w-24 text-right"
					/>
				</div>
			{/each}
		</CardContent>
	</Card>

	<div class="mt-5 flex justify-end gap-3">
		<Button variant="secondary" onclick={cancel}>Cancel</Button>
		<Button onclick={save}>Save & Close</Button>
	</div>
</div>
