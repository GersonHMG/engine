<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import type { UnlistenFn } from '@tauri-apps/api/event';
	import { computeWheelVelocities, drawWheelViz } from '$lib/rendering/wheels.js';

	let canvas: HTMLCanvasElement;
	let unlisten: UnlistenFn;

	let currentVx = $state(0);
	let currentVy = $state(0);
	let currentOmega = $state(0);

	onMount(async () => {
		draw();

		unlisten = await listen('wheel-vel-update', (event: any) => {
			const p = event.payload;
			currentVx = p.vx || 0;
			currentVy = p.vy || 0;
			currentOmega = p.omega || 0;
			draw();
		});
	});

	onDestroy(() => {
		if (unlisten) unlisten();
	});

	function draw() {
		if (!canvas) return;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;
		const wheelVels = computeWheelVelocities(currentVx, currentVy, currentOmega);
		drawWheelViz(ctx, canvas.width, canvas.height, currentVx, currentVy, currentOmega, wheelVels);
	}
</script>

<div class="flex h-screen flex-col items-center overflow-hidden bg-background p-3 text-foreground">
	<h3 class="mb-2 text-xs font-medium uppercase tracking-widest text-muted-foreground">
		Omnidirectional Robot — Wheel Velocities
	</h3>

	<canvas
		bind:this={canvas}
		width="340"
		height="340"
		class="rounded-md border border-border bg-[#111]"
	></canvas>

	<div class="mt-3 flex gap-4 font-mono text-xs text-muted-foreground">
		<span>Vx: <span class="font-bold text-foreground">{currentVx.toFixed(2)}</span></span>
		<span>Vy: <span class="font-bold text-foreground">{currentVy.toFixed(2)}</span></span>
		<span>ω: <span class="font-bold text-foreground">{currentOmega.toFixed(2)}</span></span>
	</div>
</div>
