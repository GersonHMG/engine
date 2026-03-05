<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & { children?: any } = $props();
</script>

<div class={cn('flex flex-col space-y-1.5 p-3', className)} {...restProps}>
	{@render children?.()}
</div>
