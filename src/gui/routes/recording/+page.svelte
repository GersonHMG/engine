<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { appWindow } from '@tauri-apps/api/window';
	import { Card, CardContent } from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Button } from '$lib/components/ui/button/index.js';

	let filename = $state('record.csv');
	let status = $state<'idle' | 'recording' | 'saved'>('idle');
	let startDisabled = $state(false);
	let stopDisabled = $state(true);

	async function startRecording() {
		try {
			await invoke('start_recording', { filename });
			status = 'recording';
			startDisabled = true;
			stopDisabled = false;
		} catch (e) {
			console.error(e);
			alert(String(e));
		}
	}

	async function stopRecording() {
		try {
			await invoke('stop_recording');
			status = 'saved';
			startDisabled = false;
			stopDisabled = true;
		} catch (e) {
			console.error(e);
		}
	}

	const statusColor = $derived(
		status === 'recording' ? 'text-green-500' : 'text-muted-foreground'
	);
	const statusText = $derived(
		status === 'recording' ? 'Recording...' : status === 'saved' ? 'Saved' : 'Idle'
	);
</script>

<div class="flex min-h-screen flex-col bg-background p-5 text-foreground">
	<h3 class="mb-5 text-center text-lg font-semibold">Recording</h3>

	<Card>
		<CardContent class="space-y-3 pt-4">
			<div class="flex items-center justify-between">
				<Label>Filename</Label>
				<Input type="text" bind:value={filename} class="w-40 text-right" />
			</div>
			<div class="flex gap-2">
				<Button class="flex-1" disabled={startDisabled} onclick={startRecording}>Start</Button>
				<Button variant="destructive" class="flex-1" disabled={stopDisabled} onclick={stopRecording}>Stop</Button>
			</div>
			<p class="text-xs {statusColor}">{statusText}</p>
		</CardContent>
	</Card>

	<div class="mt-5 flex justify-end">
		<Button variant="secondary" onclick={() => appWindow.close()}>Close</Button>
	</div>
</div>
