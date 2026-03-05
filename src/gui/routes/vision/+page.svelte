<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { appWindow } from '@tauri-apps/api/window';
	import { Card, CardContent } from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Button } from '$lib/components/ui/button/index.js';

	let visionIp = $state('224.5.23.2');
	let visionPort = $state(10020);

	async function reconnect() {
		try {
			await invoke('update_vision_connection', {
				ip: visionIp,
				port: visionPort
			});
			appWindow.close();
		} catch (e) {
			console.error('Failed to update vision:', e);
			alert('Failed to reconnect: ' + e);
		}
	}
</script>

<div class="flex min-h-screen flex-col bg-background p-5 text-foreground">
	<h3 class="mb-5 text-center text-lg font-semibold">Vision Connection</h3>

	<Card>
		<CardContent class="space-y-3 pt-4">
			<div class="flex items-center justify-between">
				<Label>IP</Label>
				<Input type="text" bind:value={visionIp} class="w-40 text-right" />
			</div>
			<div class="flex items-center justify-between">
				<Label>Port</Label>
				<Input type="number" bind:value={visionPort} class="w-28 text-right" />
			</div>
		</CardContent>
	</Card>

	<div class="mt-5 flex justify-end gap-3">
		<Button variant="secondary" onclick={() => appWindow.close()}>Cancel</Button>
		<Button onclick={reconnect}>Connect & Close</Button>
	</div>
</div>
