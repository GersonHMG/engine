<script lang="ts">
	import { Card, CardHeader, CardTitle, CardContent } from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import {
		manualControlMode,
		manualControlActive,
		controlTeam,
		controlRobotId,
		velScaleVx,
		velScaleVy,
		velScaleW,
		visualizeVelocities,
		gamepadStatus,
		gamepadConnected
	} from '$lib/stores/app.js';

	// Local bindings that sync with stores
	let mode = $state<'xbox' | 'keyboard'>('xbox');
	let active = $state(false);
	let team = $state(0);
	let robotId = $state(0);
	let scaleVx = $state(1.0);
	let scaleVy = $state(1.0);
	let scaleW = $state(1.0);
	let visVels = $state(false);

	// Sync to stores
	$effect(() => { manualControlMode.set(mode); });
	$effect(() => { manualControlActive.set(active); });
	$effect(() => { controlTeam.set(team); });
	$effect(() => { controlRobotId.set(robotId); });
	$effect(() => { velScaleVx.set(scaleVx); });
	$effect(() => { velScaleVy.set(scaleVy); });
	$effect(() => { velScaleW.set(scaleW); });
	$effect(() => { visualizeVelocities.set(visVels); });
</script>

<Card>
	<CardHeader>
		<CardTitle>Manual Control Robot</CardTitle>
	</CardHeader>
	<CardContent class="space-y-3">
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
