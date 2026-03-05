<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { listen, emit } from '@tauri-apps/api/event';
	import type { UnlistenFn } from '@tauri-apps/api/event';
	import {
		activeTab,
		robotsBlue,
		robotsYellow,
		ball,
		currentPPS,
		lastVisionUpdate,
		controlRobotId,
		controlTeam,
		manualControlActive,
		pushVelSample
	} from '$lib/stores/app.js';
	import { setupControlListeners } from '$lib/services/control.js';

	import FieldCanvas from '$lib/components/engine/FieldCanvas.svelte';
	import VisionPanel from '$lib/components/engine/VisionPanel.svelte';
	import RadioPanel from '$lib/components/engine/RadioPanel.svelte';
	import KalmanFilterPanel from '$lib/components/engine/KalmanFilterPanel.svelte';
	import RecordingPanel from '$lib/components/engine/RecordingPanel.svelte';
	import ManualControlPanel from '$lib/components/engine/ManualControlPanel.svelte';
	import PathFollowingPanel from '$lib/components/engine/PathFollowingPanel.svelte';
	import VelocityChartPanel from '$lib/components/engine/VelocityChartPanel.svelte';

	let tab = $state<'connection' | 'control'>('connection');
	let unlistenVision: UnlistenFn;
	let cleanupControl: (() => void) | undefined;

	$effect(() => {
		activeTab.set(tab);
	});

	onMount(async () => {
		cleanupControl = setupControlListeners();

		unlistenVision = await listen('vision-update', (event: any) => {
			const payload = event.payload;
			if (payload.robots_blue) robotsBlue.set(payload.robots_blue);
			if (payload.robots_yellow) robotsYellow.set(payload.robots_yellow);
			if (payload.ball) ball.set(payload.ball);
			if (payload.pps !== undefined) currentPPS.set(payload.pps);

			// Feed velocity chart
			let ctrlId = 0;
			let ctrlTeam = 0;
			controlRobotId.subscribe((v) => (ctrlId = v))();
			controlTeam.subscribe((v) => (ctrlTeam = v))();

			const robotList = ctrlTeam === 0 ? payload.robots_blue : payload.robots_yellow;
			const target = robotList?.find((r: any) => r.id === ctrlId);
			const cmdVx = target?.cmd_vx || 0;
			const cmdVy = target?.cmd_vy || 0;
			const cmdOmega = target?.cmd_angular || 0;
			pushVelSample(cmdVx, cmdVy, cmdOmega);

			// Broadcast to wheel-velocities window
			emit('wheel-vel-update', { vx: cmdVx, vy: cmdVy, omega: cmdOmega });

			lastVisionUpdate.set(Date.now());
		});
	});

	onDestroy(() => {
		if (unlistenVision) unlistenVision();
		if (cleanupControl) cleanupControl();
	});

	function setTab(t: 'connection' | 'control') {
		tab = t;
	}
</script>

<div class="flex h-screen flex-col overflow-hidden bg-background text-foreground">
	<!-- Top Navigation -->
	<nav class="flex h-12 shrink-0 items-center gap-1 border-b border-border bg-card px-4">
		<button
			class="flex h-full items-center gap-2 border-b-2 px-4 text-sm font-semibold transition-colors {tab === 'connection'
				? 'border-primary text-foreground'
				: 'border-transparent text-muted-foreground hover:text-foreground'}"
			onclick={() => setTab('connection')}
		>
			Connection
		</button>
		<button
			class="flex h-full items-center gap-2 border-b-2 px-4 text-sm font-semibold transition-colors {tab === 'control'
				? 'border-primary text-foreground'
				: 'border-transparent text-muted-foreground hover:text-foreground'}"
			onclick={() => setTab('control')}
		>
			Control
			{#if $manualControlActive}
				<span class="h-2.5 w-2.5 rounded-full bg-green-500 shadow-[0_0_5px_theme(colors.green.500)]"></span>
			{/if}
		</button>
	</nav>

	<!-- Content: Sidebar + Field -->
	<div class="flex flex-1 overflow-hidden">
		<!-- Sidebar -->
		<aside class="flex w-72 shrink-0 flex-col gap-3 overflow-y-auto border-r border-border bg-card p-3">
			<h3 class="border-b border-border pb-2 text-sm font-semibold">Configuration</h3>

			{#if tab === 'connection'}
				<VisionPanel />
				<RadioPanel />
				<KalmanFilterPanel />
				<RecordingPanel />
			{:else}
				<ManualControlPanel />
				<PathFollowingPanel />
				<VelocityChartPanel />
			{/if}
		</aside>

		<!-- Main Canvas -->
		<FieldCanvas />
	</div>
</div>
