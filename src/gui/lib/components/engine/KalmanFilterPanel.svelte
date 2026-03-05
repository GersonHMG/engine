<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { Card, CardHeader, CardTitle, CardContent } from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';

	let enabled = $state(true);
	let processNoiseP = $state(0.0000001);
	let processNoiseV = $state(0.0001);
	let measurementNoise = $state(0.000001);

	async function updateKF() {
		try {
			await invoke('update_tracker_config', {
				enabled,
				processNoiseP: processNoiseP,
				processNoiseV: processNoiseV,
				measurementNoise: measurementNoise
			});
		} catch (e) {
			console.error('Failed to update KF:', e);
		}
	}
</script>

<Card>
	<CardHeader>
		<CardTitle>Kalman Filter</CardTitle>
	</CardHeader>
	<CardContent class="space-y-3">
		<div class="flex items-center justify-between">
			<Label>Enabled</Label>
			<Switch bind:checked={enabled} />
		</div>
		<div class="flex items-center justify-between">
			<Label>Proc. Noise (P)</Label>
			<Input type="number" bind:value={processNoiseP} step="0.0000001" class="w-28 text-right" />
		</div>
		<div class="flex items-center justify-between">
			<Label>Proc. Noise (V)</Label>
			<Input type="number" bind:value={processNoiseV} step="0.0001" class="w-28 text-right" />
		</div>
		<div class="flex items-center justify-between">
			<Label>Meas. Noise</Label>
			<Input type="number" bind:value={measurementNoise} step="0.000001" class="w-28 text-right" />
		</div>
		<Button class="w-full" onclick={updateKF}>Update KF</Button>
	</CardContent>
</Card>
