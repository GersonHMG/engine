<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { appWindow } from '@tauri-apps/api/window';
	import { Card, CardContent } from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';

	let portName = $state('/dev/ttyUSB0');
	let baudRate = $state(115200);
	let useRadio = $state(false);

	async function updateRadio() {
		try {
			await invoke('update_radio_config', {
				useRadio: useRadio,
				portName: portName,
				baudRate: baudRate
			});
			appWindow.close();
		} catch (e) {
			console.error('Failed to update radio:', e);
			alert('Failed to update radio: ' + e);
		}
	}
</script>

<div class="flex min-h-screen flex-col bg-background p-5 text-foreground">
	<h3 class="mb-5 text-center text-lg font-semibold">Radio Configuration</h3>

	<Card>
		<CardContent class="space-y-3 pt-4">
			<div class="flex items-center justify-between">
				<Label>Port Name</Label>
				<Input type="text" bind:value={portName} class="w-40 text-right" />
			</div>
			<div class="flex items-center justify-between">
				<Label>Baud Rate</Label>
				<Input type="number" bind:value={baudRate} class="w-28 text-right" />
			</div>
			<div class="flex items-center justify-between">
				<Label>Use Radio</Label>
				<Switch bind:checked={useRadio} />
			</div>
		</CardContent>
	</Card>

	<div class="mt-5 flex justify-end gap-3">
		<Button variant="secondary" onclick={() => appWindow.close()}>Cancel</Button>
		<Button onclick={updateRadio}>Apply & Close</Button>
	</div>
</div>
