<script lang="ts">
	let { icon, label = 'Action' }: { icon: string; label?: string } = $props();
</script>

<button class="icon-btn" aria-label={label} title={label} type="button"><i class="ti {icon}" aria-hidden="true"></i></button>

<style>
	.icon-btn { width: 26px; height: 26px; display: inline-flex; align-items: center; justify-content: center; border-radius: var(--border-radius-md); border: 0.5px solid transparent; color: var(--color-text-tertiary); cursor: pointer; font-size: 14px; background: transparent; }
	.icon-btn:hover { background: var(--color-background-secondary); border-color: var(--color-border-tertiary); color: var(--color-text-primary); }
</style>
