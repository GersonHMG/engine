<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { Card, CardHeader, CardTitle, CardContent } from '$lib/components/ui/card/index.js';
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
			alert('Radio configuration updated!');
		} catch (e) {
			console.error('Failed to update radio:', e);
			alert('Failed to update radio: ' + e);
		}
	}
</script>

<Card>
	<CardHeader>
		<CardTitle>Radio Configuration</CardTitle>
	</CardHeader>
	<CardContent class="space-y-3">
		<div class="flex items-center justify-between">
			<Label>Port Name</Label>
			<Input type="text" bind:value={portName} class="w-32 text-right" />
		</div>
		<div class="flex items-center justify-between">
			<Label>Baud Rate</Label>
			<Input type="number" bind:value={baudRate} class="w-24 text-right" />
		</div>
		<div class="flex items-center justify-between">
			<Label>Use Radio</Label>
			<Switch bind:checked={useRadio} />
		</div>
		<Button class="w-full" onclick={updateRadio}>Update Radio</Button>
	</CardContent>
</Card>
