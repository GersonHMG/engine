<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { WebviewWindow } from '@tauri-apps/api/window';
	import { emit } from '@tauri-apps/api/event';
	import { Card, CardHeader, CardTitle, CardContent } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { velHistory, controlRobotId, controlTeam, robotsBlue, robotsYellow } from '$lib/stores/app.js';
	import { drawVelPlot } from '$lib/rendering/charts.js';

	let vxCanvas: HTMLCanvasElement;
	let vyCanvas: HTMLCanvasElement;
	let omegaCanvas: HTMLCanvasElement;

	let vxVal = $state('0.00');
	let vyVal = $state('0.00');
	let omegaVal = $state('0.00');

	let rafId: number;

	function drawCharts() {
		const h = $velHistory;
		if (vxCanvas) {
			const ctx = vxCanvas.getContext('2d');
			if (ctx) {
				const v = drawVelPlot(ctx, vxCanvas.width, vxCanvas.height, h.vx, '#ff4444');
				vxVal = v.toFixed(2);
			}
		}
		if (vyCanvas) {
			const ctx = vyCanvas.getContext('2d');
			if (ctx) {
				const v = drawVelPlot(ctx, vyCanvas.width, vyCanvas.height, h.vy, '#44ff44');
				vyVal = v.toFixed(2);
			}
		}
		if (omegaCanvas) {
			const ctx = omegaCanvas.getContext('2d');
			if (ctx) {
				const v = drawVelPlot(ctx, omegaCanvas.width, omegaCanvas.height, h.omega, '#4488ff');
				omegaVal = v.toFixed(2);
			}
		}
		rafId = requestAnimationFrame(drawCharts);
	}

	onMount(() => {
		rafId = requestAnimationFrame(drawCharts);
	});

	onDestroy(() => {
		if (rafId) cancelAnimationFrame(rafId);
	});

	function openWheels() {
		const webview = new WebviewWindow('wheel-velocities', {
			url: 'wheels',
			title: 'Wheel Velocities',
			width: 380,
			height: 440,
			resizable: false
		});
		webview.once('tauri://error', (e: unknown) => {
			console.error('Error creating wheel-velocities window.', e);
		});
	}
</script>

<Card>
	<CardHeader>
		<CardTitle>Velocity Command</CardTitle>
	</CardHeader>
	<CardContent class="space-y-2">
		<div class="flex items-center gap-2">
			<span class="w-5 text-right font-mono text-xs font-bold text-red-500">Vx</span>
			<canvas bind:this={vxCanvas} width="220" height="40" class="flex-1 rounded border border-border"></canvas>
			<span class="w-10 text-right font-mono text-xs text-muted-foreground">{vxVal}</span>
		</div>
		<div class="flex items-center gap-2">
			<span class="w-5 text-right font-mono text-xs font-bold text-green-500">Vy</span>
			<canvas bind:this={vyCanvas} width="220" height="40" class="flex-1 rounded border border-border"></canvas>
			<span class="w-10 text-right font-mono text-xs text-muted-foreground">{vyVal}</span>
		</div>
		<div class="flex items-center gap-2">
			<span class="w-5 text-right font-mono text-xs font-bold text-blue-500">ω</span>
			<canvas bind:this={omegaCanvas} width="220" height="40" class="flex-1 rounded border border-border"></canvas>
			<span class="w-10 text-right font-mono text-xs text-muted-foreground">{omegaVal}</span>
		</div>
		<Button variant="secondary" class="w-full" onclick={openWheels}>
			⚙ Wheel Velocities
		</Button>
	</CardContent>
</Card>
