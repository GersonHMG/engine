<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLHeadingElement> & { children?: any } = $props();
</script>

<h4
	class={cn('text-xs font-medium uppercase tracking-wide text-muted-foreground', className)}
	{...restProps}
>
	{@render children?.()}
</h4>
