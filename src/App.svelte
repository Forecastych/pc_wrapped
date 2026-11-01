<script lang="ts">
  import { onMount } from 'svelte';
  import Main from './components/Main.svelte';
  import Mini from './components/Mini.svelte';
  import { windowLabel } from './lib/api';

  let label = $state<string | null>(null);
  onMount(async () => {
    label = await windowLabel();
    if (label === 'mini') document.body.classList.add('mini');
  });
</script>

{#if label === 'mini'}
  <Mini />
{:else if label}
  <Main />
{/if}
