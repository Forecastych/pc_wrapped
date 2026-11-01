<script lang="ts">
  import { onMount } from 'svelte';
  import Main from './components/Main.svelte';
  import Mini from './components/Mini.svelte';
  import Toast from './components/Toast.svelte';
  import { windowLabel } from './lib/api';

  let label = $state<string | null>(null);
  onMount(async () => {
    label = await windowLabel();
    if (label === 'mini' || label === 'toast') document.body.classList.add(label);
  });
</script>

{#if label === 'mini'}
  <Mini />
{:else if label === 'toast'}
  <Toast />
{:else if label}
  <Main />
{/if}
