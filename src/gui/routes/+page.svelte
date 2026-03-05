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
		manualControlMode,
		velScaleVx,
		velScaleVy,
		velScaleW,
		visualizeVelocities,
		pushVelSample,
		pushPosSample
	} from '$lib/stores/app.js';
	import { setupControlListeners } from '$lib/services/control.js';

	import FieldCanvas from '$lib/components/engine/FieldCanvas.svelte';
	import FieldToolbar from '$lib/components/engine/FieldToolbar.svelte';
	import { WebviewWindow } from '@tauri-apps/api/window';
	import BottomPanel from '$lib/components/engine/BottomPanel.svelte';

	function openControlWindow() {
		const webview = new WebviewWindow('manual-control', {
			url: 'control',
			title: 'Manual Control',
			width: 380,
			height: 520,
			resizable: false
		});
		webview.once('tauri://error', (e: unknown) => {
			console.error('Error creating control window.', e);
		});
	}

	function openRadioWindow() {
		const webview = new WebviewWindow('radio-config', {
			url: 'radio',
			title: 'Radio Configuration',
			width: 380,
			height: 400,
			resizable: false
		});
		webview.once('tauri://error', (e: unknown) => {
			console.error('Error creating radio window.', e);
		});
	}

	function openKalmanWindow() {
		const webview = new WebviewWindow('kalman-config', {
			url: 'kalman',
			title: 'Kalman Filter',
			width: 380,
			height: 480,
			resizable: false
		});
		webview.once('tauri://error', (e: unknown) => {
			console.error('Error creating kalman window.', e);
		});
	}

	function openRecordingWindow() {
		const webview = new WebviewWindow('recording-config', {
			url: 'recording',
			title: 'Recording',
			width: 380,
			height: 340,
			resizable: false
		});
		webview.once('tauri://error', (e: unknown) => {
			console.error('Error creating recording window.', e);
		});
	}

	function openVisionWindow() {
		const webview = new WebviewWindow('vision-config', {
			url: 'vision',
			title: 'Vision Connection',
			width: 380,
			height: 400,
			resizable: false
		});
		webview.once('tauri://error', (e: unknown) => {
			console.error('Error creating vision window.', e);
		});
	}

	let unlistenVision: UnlistenFn;
	let unlistenControl: UnlistenFn;
	let cleanupControl: (() => void) | undefined;

	onMount(async () => {
		cleanupControl = setupControlListeners();

		unlistenControl = await listen('control-settings', (event: any) => {
			const p = event.payload;
			manualControlMode.set(p.mode);
			manualControlActive.set(p.active);
			controlTeam.set(p.team);
			controlRobotId.set(p.robotId);
			velScaleVx.set(p.scaleVx);
			velScaleVy.set(p.scaleVy);
			velScaleW.set(p.scaleW);
			visualizeVelocities.set(p.visVels);
		});

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
		if (unlistenControl) unlistenControl();
		if (cleanupControl) cleanupControl();
	});
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

	<!-- Content: Sidebar + Field -->
	<div class="flex flex-1 overflow-hidden">
		<!-- Sidebar -->
		<aside class="flex shrink-0 flex-col gap-2 overflow-y-auto border-r border-border bg-card p-2">
			<button
				class="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
				onclick={openVisionWindow}
				title="Vision Connection"
			>
				<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/></svg>
			</button>
			<button
				class="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
				onclick={openRadioWindow}
				title="Radio Configuration"
			>
				<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9"/><path d="M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.4"/><circle cx="12" cy="12" r="2"/><path d="M16.2 7.8c2.3 2.3 2.3 6.1 0 8.4"/><path d="M19.1 4.9C23 8.8 23 15.1 19.1 19"/></svg>
			</button>
			<button
				class="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
				onclick={openKalmanWindow}
				title="Kalman Filter"
			>
				<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3v18h18"/><path d="m19 9-5 5-4-4-3 3"/></svg>
			</button>
			<button
				class="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
				onclick={openRecordingWindow}
				title="Recording"
			>
				<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="4" fill="currentColor"/></svg>
			</button>
			<button
				class="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
				onclick={openControlWindow}
				title="Manual Control"
			>
				<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="6" width="20" height="12" rx="2"/><circle cx="12" cy="12" r="2"/><path d="M6 12h.01"/><path d="M18 12h.01"/></svg>
			</button>
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
