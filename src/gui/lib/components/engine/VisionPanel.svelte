<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import { Card, CardHeader, CardTitle, CardContent } from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { currentPPS, ppsHistory, lastVisionUpdate } from '$lib/stores/app.js';
	import { drawPpsGraph } from '$lib/rendering/charts.js';

	let visionIp = $state('224.5.23.2');
	let visionPort = $state(10020);
	let connected = $state(false);
	let statusText = $state('Disconnected');

	let ppsCanvas: HTMLCanvasElement;
	let ppsInterval: ReturnType<typeof setInterval>;

	$effect(() => {
		const ts = $lastVisionUpdate;
		connected = Date.now() - ts < 1000;
		statusText = connected ? `Connected (${$currentPPS} PPS)` : 'Disconnected';
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
			statusText = 'Connecting...';
		} catch (e) {
			console.error('Failed to update vision:', e);
			alert('Failed to reconnect: ' + e);
		}
	}
</script>

<Card>
	<CardHeader>
		<CardTitle>Vision Connection</CardTitle>
	</CardHeader>
	<CardContent class="space-y-3">
		<div class="flex items-center justify-between">
			<Label>IP</Label>
			<Input type="text" bind:value={visionIp} class="w-32 text-right" />
		</div>
		<div class="flex items-center justify-between">
			<Label>Port</Label>
			<Input type="number" bind:value={visionPort} class="w-24 text-right" />
		</div>
		<div class="flex items-center justify-between">
			<Label>Status</Label>
			<Badge variant={connected ? 'default' : 'destructive'}>
				{statusText}
			</Badge>
		</div>
		<div class="space-y-1">
			<Label>PPS History</Label>
			<canvas
				bind:this={ppsCanvas}
				width="200"
				height="40"
				class="w-full rounded border border-border bg-[#222]"
			></canvas>
		</div>
		<Button class="w-full" onclick={reconnect}>Reconnect</Button>
	</CardContent>
</Card>
