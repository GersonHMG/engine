<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { Button } from '$lib/components/ui/button/index.js';
	import {
		velHistory,
		posHistory,
		controlRobotId,
		controlTeam,
		pathTraceMode,
		visualizeVelocities
	} from '$lib/stores/app.js';
	import { drawVelPlot } from '$lib/rendering/charts.js';

	let vxCanvas: HTMLCanvasElement;
	let vyCanvas: HTMLCanvasElement;
	let omegaCanvas: HTMLCanvasElement;
	let posXCanvas: HTMLCanvasElement;
	let posYCanvas: HTMLCanvasElement;
	let posThetaCanvas: HTMLCanvasElement;

	let traceOn = $state(false);
	let vectorsOn = $state(false);
	let capturing = $state(false);

	let rafId: number;

	$effect(() => {
		traceOn = $pathTraceMode;
	});

	$effect(() => {
		vectorsOn = $visualizeVelocities;
	});

	function toggleTrace() {
		pathTraceMode.update((v) => !v);
	}

	function toggleVectors() {
		visualizeVelocities.update((v) => !v);
	}

	function toggleCapture() {
		capturing = !capturing;
	}

	function drawPositionPlot(
		ctx: CanvasRenderingContext2D,
		w: number,
		h: number,
		data: number[],
		color: string,
		label: string
	) {
		ctx.clearRect(0, 0, w, h);

		let minVal = Infinity;
		let maxVal = -Infinity;
		for (let i = 0; i < data.length; i++) {
			if (data[i] < minVal) minVal = data[i];
			if (data[i] > maxVal) maxVal = data[i];
		}

		const range = maxVal - minVal;
		const padding = range < 0.01 ? 1 : range * 0.15;
		const yMin = minVal - padding;
		const yMax = maxVal + padding;

		// Background
		ctx.fillStyle = '#1a1a2e';
		ctx.fillRect(0, 0, w, h);

		// Grid line at center
		const centerVal = (yMin + yMax) / 2;
		const centerY = h - ((centerVal - yMin) / (yMax - yMin)) * h;
		ctx.strokeStyle = '#333';
		ctx.lineWidth = 1;
		ctx.setLineDash([3, 3]);
		ctx.beginPath();
		ctx.moveTo(0, centerY);
		ctx.lineTo(w, centerY);
		ctx.stroke();
		ctx.setLineDash([]);

		// Y-axis labels
		ctx.fillStyle = '#555';
		ctx.font = '8px monospace';
		ctx.textAlign = 'left';
		ctx.fillText(yMax.toFixed(1), 1, 8);
		ctx.fillText(yMin.toFixed(1), 1, h - 2);

		// Label
		ctx.fillStyle = color;
		ctx.font = 'bold 9px monospace';
		ctx.textAlign = 'right';
		ctx.fillText(label, w - 3, 10);

		// Data line
		ctx.strokeStyle = color;
		ctx.lineWidth = 1.5;
		ctx.beginPath();
		for (let i = 0; i < data.length; i++) {
			const x = (i / (data.length - 1)) * w;
			const y = h - ((data[i] - yMin) / (yMax - yMin)) * h;
			if (i === 0) ctx.moveTo(x, y);
			else ctx.lineTo(x, y);
		}
		ctx.stroke();

		// Current value
		const last = data[data.length - 1];
		ctx.fillStyle = '#aaa';
		ctx.font = '9px monospace';
		ctx.textAlign = 'right';
		ctx.fillText(last.toFixed(2), w - 3, h - 3);
	}

	function draw() {
		if (capturing) {
			const vel = get(velHistory);
			const pos = get(posHistory);

			if (vxCanvas) {
				const ctx = vxCanvas.getContext('2d');
				if (ctx) drawVelPlot(ctx, vxCanvas.width, vxCanvas.height, vel.vx, '#ff4444');
			}
			if (vyCanvas) {
				const ctx = vyCanvas.getContext('2d');
				if (ctx) drawVelPlot(ctx, vyCanvas.width, vyCanvas.height, vel.vy, '#44ff44');
			}
			if (omegaCanvas) {
				const ctx = omegaCanvas.getContext('2d');
				if (ctx) drawVelPlot(ctx, omegaCanvas.width, omegaCanvas.height, vel.omega, '#4488ff');
			}
			if (posXCanvas) {
				const ctx = posXCanvas.getContext('2d');
				if (ctx) drawPositionPlot(ctx, posXCanvas.width, posXCanvas.height, pos.x, '#ff8844', 'X');
			}
			if (posYCanvas) {
				const ctx = posYCanvas.getContext('2d');
				if (ctx) drawPositionPlot(ctx, posYCanvas.width, posYCanvas.height, pos.y, '#44ddff', 'Y');
			}
			if (posThetaCanvas) {
				const ctx = posThetaCanvas.getContext('2d');
				if (ctx)
					drawPositionPlot(ctx, posThetaCanvas.width, posThetaCanvas.height, pos.theta, '#dd88ff', 'θ');
			}
		}

		rafId = requestAnimationFrame(draw);
	}

	onMount(() => {
		rafId = requestAnimationFrame(draw);
	});

	onDestroy(() => {
		if (rafId) cancelAnimationFrame(rafId);
	});
</script>

<div
	class="flex h-44 shrink-0 items-stretch gap-2 border-t border-border bg-card px-3 py-2"
>
	<!-- Controls column -->
	<div class="flex w-24 shrink-0 flex-col justify-center gap-1.5">
		<span class="text-center text-[10px] font-semibold text-muted-foreground">
			Robot {$controlRobotId} · {$controlTeam === 0 ? 'Blue' : 'Yellow'}
		</span>
		<Button
			variant={capturing ? 'default' : 'secondary'}
			size="sm"
			class="h-7 text-[10px]"
			onclick={toggleCapture}
		>
			{capturing ? 'Capture ON' : 'Capture OFF'}
		</Button>
		<Button
			variant={traceOn ? 'default' : 'secondary'}
			size="sm"
			class="h-7 text-[10px]"
			onclick={toggleTrace}
		>
			{traceOn ? 'Trace ON' : 'Trace OFF'}
		</Button>
		<Button
			variant={vectorsOn ? 'default' : 'secondary'}
			size="sm"
			class="h-7 text-[10px]"
			onclick={toggleVectors}
		>
			{vectorsOn ? 'Vectors ON' : 'Vectors OFF'}
		</Button>
	</div>

	<!-- Separator -->
	<div class="w-px shrink-0 bg-border"></div>

	<!-- Plots: 3 columns, each with vel on top + pos below -->
	<div class="flex min-w-0 flex-1 gap-1">
		<!-- Vx / X column -->
		<div class="flex flex-1 flex-col gap-0.5">
			<div class="flex flex-1 flex-col">
				<span class="text-[9px] font-bold text-red-400">Vx</span>
				<canvas
					bind:this={vxCanvas}
					width="200"
					height="36"
					class="h-full w-full rounded border border-border"
				></canvas>
			</div>
			<div class="flex flex-1 flex-col">
				<span class="text-[9px] font-bold text-orange-400">X</span>
				<canvas
					bind:this={posXCanvas}
					width="200"
					height="36"
					class="h-full w-full rounded border border-border"
				></canvas>
			</div>
		</div>

		<!-- Vy / Y column -->
		<div class="flex flex-1 flex-col gap-0.5">
			<div class="flex flex-1 flex-col">
				<span class="text-[9px] font-bold text-green-400">Vy</span>
				<canvas
					bind:this={vyCanvas}
					width="200"
					height="36"
					class="h-full w-full rounded border border-border"
				></canvas>
			</div>
			<div class="flex flex-1 flex-col">
				<span class="text-[9px] font-bold text-cyan-400">Y</span>
				<canvas
					bind:this={posYCanvas}
					width="200"
					height="36"
					class="h-full w-full rounded border border-border"
				></canvas>
			</div>
		</div>

		<!-- ω / θ column -->
		<div class="flex flex-1 flex-col gap-0.5">
			<div class="flex flex-1 flex-col">
				<span class="text-[9px] font-bold text-blue-400">ω</span>
				<canvas
					bind:this={omegaCanvas}
					width="200"
					height="36"
					class="h-full w-full rounded border border-border"
				></canvas>
			</div>
			<div class="flex flex-1 flex-col">
				<span class="text-[9px] font-bold text-purple-400">θ</span>
				<canvas
					bind:this={posThetaCanvas}
					width="200"
					height="36"
					class="h-full w-full rounded border border-border"
				></canvas>
			</div>
		</div>
	</div>
</div>
