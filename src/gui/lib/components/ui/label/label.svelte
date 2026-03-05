<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { HTMLLabelAttributes } from 'svelte/elements';

	let {
		class: className,
		children,
		...restProps
	}: HTMLLabelAttributes & { children?: any } = $props();
</script>

<label
	class={cn(
		'text-xs font-medium leading-none text-muted-foreground peer-disabled:cursor-not-allowed peer-disabled:opacity-70',
		className
	)}
	{...restProps}
>
	{@render children?.()}
</label>
