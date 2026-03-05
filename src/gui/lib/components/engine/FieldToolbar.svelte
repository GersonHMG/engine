<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import { open } from '@tauri-apps/api/dialog';
	import { Button } from '$lib/components/ui/button/index.js';
	import { scriptPath, scriptStatus, currentPPS, ppsHistory } from '$lib/stores/app.js';
	import { drawPpsGraph } from '$lib/rendering/charts.js';

	let ppsCanvas: HTMLCanvasElement;
	let ppsInterval: ReturnType<typeof setInterval>;

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

	async function selectScript() {
		const selected = await open({
			filters: [{ name: 'Lua Scripts', extensions: ['lua'] }],
			multiple: false
		});
		if (selected && typeof selected === 'string') {
			scriptPath.set(selected);
			try {
				await invoke('load_script', { path: selected });
				scriptStatus.set('loaded');
			} catch (e) {
				console.error('Failed to load script:', e);
				scriptStatus.set('error');
			}
		}
	}

	async function playScript() {
		try {
			await invoke('resume_script');
			scriptStatus.set('running');
		} catch (e) {
			console.error('Failed to resume script:', e);
		}
	}

	async function stopScript() {
		try {
			await invoke('pause_script');
			scriptStatus.set('paused');
		} catch (e) {
			console.error('Failed to pause script:', e);
		}
	}

	function filename(path: string): string {
		if (!path) return 'No script';
		const parts = path.replace(/\\/g, '/').split('/');
		return parts[parts.length - 1];
	}
</script>

<div class="flex h-9 shrink-0 items-center gap-2 border-b border-border bg-card px-3">
	<!-- Load Script -->
	<Button
		variant="ghost"
		size="sm"
		class="h-7 w-7 p-0 text-muted-foreground hover:text-foreground"
		onclick={selectScript}
		title="Load Lua Script"
	>
		<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/></svg>
	</Button>

	<!-- Play -->
	<Button
		variant="ghost"
		size="sm"
		class="h-7 w-7 p-0 text-muted-foreground hover:text-green-500"
		onclick={playScript}
		disabled={!$scriptPath}
		title="Play Script"
	>
		<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><polygon points="5 3 19 12 5 21 5 3"/></svg>
	</Button>

	<!-- Stop -->
	<Button
		variant="ghost"
		size="sm"
		class="h-7 w-7 p-0 text-muted-foreground hover:text-red-500"
		onclick={stopScript}
		disabled={!$scriptPath}
		title="Stop Script"
	>
		<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><rect x="4" y="4" width="16" height="16" rx="2"/></svg>
	</Button>

	<!-- Separator -->
	<div class="h-4 w-px bg-border"></div>

	<!-- Script filename -->
	<span class="max-w-[200px] truncate text-xs text-muted-foreground" title={$scriptPath}>
		{filename($scriptPath)}
	</span>

	<!-- Spacer to push PPS chart to the right -->
	<div class="flex-1"></div>

	<!-- PPS Chart -->
	<canvas
		bind:this={ppsCanvas}
		width="120"
		height="24"
		class="rounded border border-border bg-[#222]"
	></canvas>
</div>
