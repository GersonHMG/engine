<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { Card, CardHeader, CardTitle, CardContent } from '$lib/components/ui/card/index.js';
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
		status === 'recording' ? 'text-green-500' : status === 'saved' ? 'text-muted-foreground' : 'text-muted-foreground'
	);
	const statusText = $derived(
		status === 'recording' ? 'Recording...' : status === 'saved' ? 'Saved' : 'Idle'
	);
</script>

<Card>
	<CardHeader>
		<CardTitle>Recording</CardTitle>
	</CardHeader>
	<CardContent class="space-y-3">
		<Input type="text" bind:value={filename} class="w-full" />
		<div class="flex gap-2">
			<Button class="flex-1" disabled={startDisabled} onclick={startRecording}>Start</Button>
			<Button variant="destructive" class="flex-1" disabled={stopDisabled} onclick={stopRecording}
				>Stop</Button
			>
		</div>
		<p class="text-xs {statusColor}">{statusText}</p>
	</CardContent>
</Card>
