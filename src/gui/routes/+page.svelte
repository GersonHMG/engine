<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { listen, emit } from '@tauri-apps/api/event';
	import type { UnlistenFn } from '@tauri-apps/api/event';
	import { appWindow } from '@tauri-apps/api/window';
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
		pushVelSample,
		pushPosSample
	} from '$lib/stores/app.js';
	import { setupControlListeners } from '$lib/services/control.js';

	import FieldCanvas from '$lib/components/engine/FieldCanvas.svelte';
	import FieldToolbar from '$lib/components/engine/FieldToolbar.svelte';
	import RadioPanel from '$lib/components/engine/RadioPanel.svelte';
	import KalmanFilterPanel from '$lib/components/engine/KalmanFilterPanel.svelte';
	import RecordingPanel from '$lib/components/engine/RecordingPanel.svelte';
	import ManualControlPanel from '$lib/components/engine/ManualControlPanel.svelte';
	import PathFollowingPanel from '$lib/components/engine/PathFollowingPanel.svelte';
	import ScriptPanel from '$lib/components/engine/ScriptPanel.svelte';
	import BottomPanel from '$lib/components/engine/BottomPanel.svelte';

	let tab = $state<'connection' | 'control' | 'script'>('connection');
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

			// Push position sample for bottom panel
			const posX = target?.x || 0;
			const posY = target?.y || 0;
			const posTheta = target?.theta || 0;
			pushPosSample(posX, posY, posTheta);

			// Broadcast to wheel-velocities window
			emit('wheel-vel-update', { vx: cmdVx, vy: cmdVy, omega: cmdOmega });

			lastVisionUpdate.set(Date.now());
		});
	});

	onDestroy(() => {
		if (unlistenVision) unlistenVision();
		if (cleanupControl) cleanupControl();
	});

	function setTab(t: 'connection' | 'control' | 'script') {
		tab = t;
	}
</script>

<div class="flex h-screen flex-col overflow-hidden rounded-lg border border-border bg-background text-foreground">
	<!-- Custom Title Bar -->
	<div class="flex h-8 shrink-0 items-center justify-between bg-card" data-tauri-drag-region>
		<span class="pointer-events-none select-none pl-3 text-xs font-medium text-muted-foreground" data-tauri-drag-region>Sysmic Engine</span>
		<div class="flex h-full">
			<button
				class="inline-flex h-full w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-muted"
				onclick={() => appWindow.minimize()}
				aria-label="Minimize"
			>
				<svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" fill="currentColor"/></svg>
			</button>
			<button
				class="inline-flex h-full w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-muted"
				onclick={() => appWindow.toggleMaximize()}
				aria-label="Maximize"
			>
				<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1"><rect x="0.5" y="0.5" width="9" height="9"/></svg>
			</button>
			<button
				class="inline-flex h-full w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-red-600 hover:text-white"
				onclick={() => appWindow.close()}
				aria-label="Close"
			>
				<svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" stroke-width="1.2"><line x1="0" y1="0" x2="10" y2="10"/><line x1="10" y1="0" x2="0" y2="10"/></svg>
			</button>
		</div>
	</div>

	<!-- Top Navigation -->
	<nav class="flex h-10 shrink-0 items-center gap-1 border-b border-border bg-card px-4">
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
		<button
			class="flex h-full items-center gap-2 border-b-2 px-4 text-sm font-semibold transition-colors {tab === 'script'
				? 'border-primary text-foreground'
				: 'border-transparent text-muted-foreground hover:text-foreground'}"
			onclick={() => setTab('script')}
		>
			Script
		</button>
	</nav>

	<!-- Content: Sidebar + Field -->
	<div class="flex flex-1 overflow-hidden">
		<!-- Sidebar -->
		<aside class="flex w-72 shrink-0 flex-col gap-3 overflow-y-auto border-r border-border bg-card p-3">
			<h3 class="border-b border-border pb-2 text-sm font-semibold">Configuration</h3>

			{#if tab === 'connection'}
				<RadioPanel />
				<KalmanFilterPanel />
				<RecordingPanel />
			{:else if tab === 'control'}
				<ManualControlPanel />
				<PathFollowingPanel />
			{:else if tab === 'script'}
				<ScriptPanel />
			{/if}
		</aside>

		<!-- Main Canvas -->
		<div class="flex flex-1 flex-col overflow-hidden">
			<FieldToolbar />
			<FieldCanvas />
		</div>
	</div>

	<!-- Bottom Panel -->
	<BottomPanel />
</div>
