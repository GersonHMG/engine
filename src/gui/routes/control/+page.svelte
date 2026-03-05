<script lang="ts">
	import { appWindow } from '@tauri-apps/api/window';
	import { emit } from '@tauri-apps/api/event';
	import { Card, CardContent } from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { gamepadStatus, gamepadConnected } from '$lib/stores/app.js';

	let mode = $state<'xbox' | 'keyboard'>('xbox');
	let active = $state(false);
	let team = $state(0);
	let robotId = $state(0);
	let scaleVx = $state(1.0);
	let scaleVy = $state(1.0);
	let scaleW = $state(1.0);
	let visVels = $state(false);

	function broadcast() {
		emit('control-settings', {
			mode, active, team, robotId, scaleVx, scaleVy, scaleW, visVels
		});
	}

	$effect(() => { mode; active; team; robotId; scaleVx; scaleVy; scaleW; visVels; broadcast(); });
</script>

<div class="flex min-h-screen flex-col bg-background p-5 text-foreground">
	<h3 class="mb-5 text-center text-lg font-semibold">Manual Control</h3>

	<Card>
		<CardContent class="space-y-3 pt-4">
			<div class="flex items-center justify-between">
				<Label>Mode</Label>
				<select
					bind:value={mode}
					class="h-8 rounded-md border border-input bg-background px-2 text-sm"
				>
					<option value="xbox">Xbox</option>
					<option value="keyboard">Keyboard</option>
				</select>
			</div>
			<div class="flex items-center justify-between">
				<Label>Active</Label>
				<Switch bind:checked={active} />
			</div>
			<div class="flex items-center justify-between">
				<Label>Team</Label>
				<select
					bind:value={team}
					class="h-8 rounded-md border border-input bg-background px-2 text-sm"
				>
					<option value={0}>Blue</option>
					<option value={1}>Yellow</option>
				</select>
			</div>
			<div class="flex items-center justify-between">
				<Label>Robot ID</Label>
				<Input type="number" bind:value={robotId} min={0} max={15} class="w-16 text-right" />
			</div>
			<div class="flex items-center justify-between">
				<Label>Scale Vx</Label>
				<Input type="number" bind:value={scaleVx} step={0.1} class="w-16 text-right" />
			</div>
			<div class="flex items-center justify-between">
				<Label>Scale Vy</Label>
				<Input type="number" bind:value={scaleVy} step={0.1} class="w-16 text-right" />
			</div>
			<div class="flex items-center justify-between">
				<Label>Scale ω</Label>
				<Input type="number" bind:value={scaleW} step={0.1} class="w-16 text-right" />
			</div>
			<div class="flex items-center justify-between">
				<Label>Vis. Velocities</Label>
				<Switch bind:checked={visVels} />
			</div>
			<p class="text-xs" class:text-green-500={$gamepadConnected} class:text-muted-foreground={!$gamepadConnected}>
				{$gamepadStatus}
			</p>
		</CardContent>
	</Card>

	<div class="mt-5 flex justify-end">
		<button
			class="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
			onclick={() => appWindow.close()}
		>
			Close
		</button>
	</div>
</div>
