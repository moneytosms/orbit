<script lang="ts">
	import { cn, type WithElementRef } from "$lib/utils.js";
	import type { HTMLInputAttributes, HTMLInputTypeAttribute } from "svelte/elements";

	type InputType = Exclude<HTMLInputTypeAttribute, "file">;

	type Props = WithElementRef<
		Omit<HTMLInputAttributes, "type"> &
			({ type: "file"; files?: FileList | null } | { type?: InputType; files?: undefined })
	>;

	let {
		ref = $bindable(null),
		value = $bindable(),
		type,
		files = $bindable(),
		class: className,
		"data-slot": dataSlot = "input",
		...restProps
	}: Props = $props();
</script>

{#if type === "file"}
	<input
		bind:this={ref}
		data-slot={dataSlot}
		class={cn(
			"h-[34px] w-full min-w-0 rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none transition-colors placeholder:text-muted focus-visible:border-accent-line focus-visible:shadow-[0_0_0_3px_var(--accent-soft)] aria-invalid:border-danger disabled:cursor-not-allowed disabled:bg-sunk disabled:text-muted",
			className
		)}
		type="file"
		bind:files
		bind:value
		{...restProps}
	/>
{:else}
	<input
		bind:this={ref}
		data-slot={dataSlot}
		class={cn(
			"h-[34px] w-full min-w-0 rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none transition-colors placeholder:text-muted focus-visible:border-accent-line focus-visible:shadow-[0_0_0_3px_var(--accent-soft)] aria-invalid:border-danger disabled:cursor-not-allowed disabled:bg-sunk disabled:text-muted",
			className
		)}
		{type}
		bind:value
		{...restProps}
	/>
{/if}
