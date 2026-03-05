<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import {
		robotsBlue,
		robotsYellow,
		ball,
		lastVisionUpdate,
		pathPoints,
		pathTraceMode,
		pathRobotId,
		pathTeam,
		pathDrawMode,
		visualizeVelocities,
		robotTrace
	} from '$lib/stores/app.js';
	import {
		type Viewport,
		drawField,
		drawRobot,
		drawBall,
		drawPath,
		drawRobotTrace,
		drawDisconnectedOverlay,
		screenToField,
		screenToFieldMm
	} from '$lib/rendering/field.js';
	import { pollManualControl } from '$lib/services/control.js';

	let canvas: HTMLCanvasElement;
	let container: HTMLDivElement;
	let mouseCoords = $state('0, 0');

	let scale = 0.08;
	const MIN_SCALE = 0.01;
	const MAX_SCALE = 0.5;

	let panX = 0;
	let panY = 0;
	let isDragging = false;
	let didDrag = false;
	let lastMouseX = 0;
	let lastMouseY = 0;
	let rafId: number;

	// Trace state
	let lastTraceId = -1;
	let lastTraceTeam = -1;

	function getViewport(): Viewport {
		return {
			width: canvas.width,
			height: canvas.height,
			panX,
			panY,
			scale
		};
	}

	function resize() {
		if (!canvas || !container) return;
		canvas.width = container.clientWidth;
		canvas.height = container.clientHeight;
	}

	function updateMouseCoords(clientX: number, clientY: number) {
		if (!canvas) return;
		const vp = getViewport();
		const rect = canvas.getBoundingClientRect();
		const pos = screenToFieldMm(vp, clientX, clientY, rect);
		mouseCoords = `${pos.x}, ${pos.y}`;
	}

	function onWheel(e: WheelEvent) {
		e.preventDefault();
		const zoomFactor = 1.1;
		if (e.deltaY < 0) {
			scale = Math.min(MAX_SCALE, scale * zoomFactor);
		} else {
			scale = Math.max(MIN_SCALE, scale / zoomFactor);
		}
		updateMouseCoords(e.clientX, e.clientY);
	}

	function onMouseDown(e: MouseEvent) {
		isDragging = true;
		didDrag = false;
		lastMouseX = e.clientX;
		lastMouseY = e.clientY;
		if (canvas) canvas.style.cursor = 'grabbing';
	}

	function onMouseMove(e: MouseEvent) {
		updateMouseCoords(e.clientX, e.clientY);
		if (!isDragging) return;

		const dx = e.clientX - lastMouseX;
		const dy = e.clientY - lastMouseY;

		if (Math.abs(dx) > 2 || Math.abs(dy) > 2) {
			didDrag = true;
		}

		panX += dx;
		panY += dy;
		lastMouseX = e.clientX;
		lastMouseY = e.clientY;
	}

	function onMouseUp(e: MouseEvent) {
		if (isDragging && !didDrag && e.target === canvas) {
			const vp = getViewport();
			const rect = canvas.getBoundingClientRect();
			const pos = screenToField(vp, e.clientX, e.clientY, rect);

			if (get(pathDrawMode)) {
				pathPoints.update((pts) => [...pts, pos]);
			}
		}
		isDragging = false;
		if (canvas) canvas.style.cursor = 'grab';
	}

	function zoomIn() {
		scale = Math.min(MAX_SCALE, scale * 1.2);
	}

	function zoomOut() {
		scale = Math.max(MIN_SCALE, scale / 1.2);
	}

	function loop() {
		if (!canvas) {
			rafId = requestAnimationFrame(loop);
			return;
		}

		const ctx = canvas.getContext('2d');
		if (!ctx) {
			rafId = requestAnimationFrame(loop);
			return;
		}

		pollManualControl();

		const vp = getViewport();
		const blue = get(robotsBlue);
		const yellow = get(robotsYellow);
		const b = get(ball);
		const points = get(pathPoints);
		const visVels = get(visualizeVelocities);
		const isTraceEnabled = get(pathTraceMode);
		const traceId = get(pathRobotId);
		const traceTeam = get(pathTeam);

		// Clear trace if disabled or target changed
		if (!isTraceEnabled || traceId !== lastTraceId || traceTeam !== lastTraceTeam) {
			robotTrace.set([]);
		}
		lastTraceId = traceId;
		lastTraceTeam = traceTeam;

		drawField(ctx, vp);
		drawPath(ctx, vp, points);

		blue.forEach((r) => {
			drawRobot(
				ctx,
				vp,
				r.x,
				r.y,
				r.theta,
				'blue',
				r.id,
				visVels ? r.vx || 0 : 0,
				visVels ? r.vy || 0 : 0,
				visVels && r.cmd_vx ? r.cmd_vx : 0,
				visVels && r.cmd_vy ? r.cmd_vy : 0
			);
			if (isTraceEnabled && traceTeam === 0 && r.id === traceId) {
				robotTrace.update((t) => [...t, { x: r.x, y: r.y }]);
			}
		});

		yellow.forEach((r) => {
			drawRobot(
				ctx,
				vp,
				r.x,
				r.y,
				r.theta,
				'yellow',
				r.id,
				visVels ? r.vx || 0 : 0,
				visVels ? r.vy || 0 : 0,
				visVels && r.cmd_vx ? r.cmd_vx : 0,
				visVels && r.cmd_vy ? r.cmd_vy : 0
			);
			if (isTraceEnabled && traceTeam === 1 && r.id === traceId) {
				robotTrace.update((t) => [...t, { x: r.x, y: r.y }]);
			}
		});

		if (isTraceEnabled) {
			const trace = get(robotTrace);
			if (trace.length > 0) {
				drawRobotTrace(ctx, vp, trace);
			}
		}

		drawBall(ctx, vp, b.x, b.y);

		// Vision disconnect overlay
		if (Date.now() - get(lastVisionUpdate) > 1000) {
			drawDisconnectedOverlay(ctx, canvas.width, canvas.height);
		}

		rafId = requestAnimationFrame(loop);
	}

	onMount(() => {
		resize();
		if (canvas) canvas.style.cursor = 'grab';
		window.addEventListener('mousemove', onMouseMove);
		window.addEventListener('mouseup', onMouseUp);
		window.addEventListener('resize', resize);
		rafId = requestAnimationFrame(loop);
	});

	onDestroy(() => {
		if (rafId) cancelAnimationFrame(rafId);
		window.removeEventListener('mousemove', onMouseMove);
		window.removeEventListener('mouseup', onMouseUp);
		window.removeEventListener('resize', resize);
	});
</script>

<div bind:this={container} class="relative flex-1 overflow-hidden bg-[#111]">
	<canvas
		bind:this={canvas}
		onwheel={onWheel}
		onmousedown={onMouseDown}
		class="h-full w-full"
		style="background-color: #A9A9A9;"
	></canvas>

	<!-- Mouse coords overlay -->
	<div
		class="pointer-events-none absolute left-4 top-4 rounded bg-black/50 px-2.5 py-1 font-mono text-sm text-white"
	>
		{mouseCoords}
	</div>

	<!-- Zoom controls -->
	<div class="absolute bottom-4 right-4 flex gap-1.5">
		<button
			onclick={zoomOut}
			class="flex h-8 w-8 items-center justify-center rounded-full bg-white/10 text-lg text-white hover:bg-white/20"
		>
			-
		</button>
		<button
			onclick={zoomIn}
			class="flex h-8 w-8 items-center justify-center rounded-full bg-white/10 text-lg text-white hover:bg-white/20"
		>
			+
		</button>
	</div>
</div>
