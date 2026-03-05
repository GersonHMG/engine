<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { appWindow } from '@tauri-apps/api/window';
	import { Card, CardContent } from '$lib/components/ui/card/index.js';
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
			appWindow.close();
		} catch (e) {
			console.error('Failed to update KF:', e);
			alert('Failed to update KF: ' + e);
		}
	}
</script>

<div class="flex min-h-screen flex-col bg-background p-5 text-foreground">
	<h3 class="mb-5 text-center text-lg font-semibold">Kalman Filter</h3>

	<Card>
		<CardContent class="space-y-3 pt-4">
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
		</CardContent>
	</Card>

	<div class="mt-5 flex justify-end gap-3">
		<Button variant="secondary" onclick={() => appWindow.close()}>Cancel</Button>
		<Button onclick={updateKF}>Apply & Close</Button>
	</div>
</div>
