<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { currentPPS, ppsHistory, lastVisionUpdate } from '$lib/stores/app.js';
	import { drawPpsGraph } from '$lib/rendering/charts.js';

	let visionIp = $state('224.5.23.2');
	let visionPort = $state(10020);
	let popupOpen = $state(false);
	let popupEl: HTMLDivElement;

	let connected = $state(false);
	let statusText = $state('Disconnected');

	let ppsCanvas: HTMLCanvasElement;
	let ppsInterval: ReturnType<typeof setInterval>;

	$effect(() => {
		const ts = $lastVisionUpdate;
		connected = Date.now() - ts < 1000;
		statusText = connected ? `${$currentPPS} PPS` : 'No signal';
	});

	onMount(() => {
		ppsInterval = setInterval(() => {
			ppsHistory.update((h) => {
				const next = [...h.slice(1), $currentPPS];
				return next;
			});
			if (ppsCanvas) {
				const ctx = ppsCanvas.getContext('2d');
				if (ctx) drawPpsGraph(ctx, ppsCanvas.width, ppsCanvas.height, $ppsHistory);
			}
		}, 500);

		function handleClickOutside(e: MouseEvent) {
			if (popupOpen && popupEl && !popupEl.contains(e.target as Node)) {
				popupOpen = false;
			}
		}
		document.addEventListener('mousedown', handleClickOutside);
		return () => document.removeEventListener('mousedown', handleClickOutside);
	});

	onDestroy(() => {
		if (ppsInterval) clearInterval(ppsInterval);
	});

	async function reconnect() {
		try {
			await invoke('update_vision_connection', {
				ip: visionIp,
				port: visionPort
			});
			popupOpen = false;
		} catch (e) {
			console.error('Failed to update vision:', e);
		}
	}
</script>

<div class="flex h-9 shrink-0 items-center gap-3 border-b border-border bg-card px-3">
	<!-- Vision Settings Button + Popup -->
	<div class="relative" bind:this={popupEl}>
		<Button
			variant="ghost"
			size="sm"
			class="h-7 gap-1.5 px-2 text-xs"
			onclick={() => (popupOpen = !popupOpen)}
		>
			<svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<circle cx="12" cy="12" r="3"/>
				<path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
			</svg>
			Vision
		</Button>

		{#if popupOpen}
			<div class="absolute left-0 top-full z-50 mt-1 w-64 rounded-md border border-border bg-card p-3 shadow-lg">
				<h4 class="mb-2 text-xs font-semibold text-muted-foreground">Vision Connection</h4>
				<div class="space-y-2">
					<div class="flex items-center justify-between gap-2">
						<Label class="text-xs">IP</Label>
						<Input type="text" bind:value={visionIp} class="h-7 w-36 text-xs" />
					</div>
					<div class="flex items-center justify-between gap-2">
						<Label class="text-xs">Port</Label>
						<Input type="number" bind:value={visionPort} class="h-7 w-24 text-xs" />
					</div>
					<Button size="sm" class="h-7 w-full text-xs" onclick={reconnect}>
						Reconnect
					</Button>
				</div>
			</div>
		{/if}
	</div>

	<!-- Separator -->
	<div class="h-4 w-px bg-border"></div>

	<!-- Status Badge -->
	<Badge variant={connected ? 'default' : 'destructive'} class="h-5 text-[10px]">
		{statusText}
	</Badge>

	<!-- Inline PPS Chart -->
	<canvas
		bind:this={ppsCanvas}
		width="120"
		height="24"
		class="rounded border border-border bg-[#222]"
	></canvas>
</div>
