<script>
  import ProbePanel from './lib/ProbePanel.svelte';
  import FlashPanel from './lib/FlashPanel.svelte';
  import RttPanel from './lib/RttPanel.svelte';
  import StatusBar from './lib/StatusBar.svelte';

  let connected = $state(false);
  let targetName = $state('');
  let statusMessage = $state('Disconnected');

  function handleConnected(event) {
    connected = true;
    targetName = event.detail;
    statusMessage = `Connected to ${event.detail}`;
  }

  function handleDisconnected() {
    connected = false;
    targetName = '';
    statusMessage = 'Disconnected';
  }

  function handleStatus(event) {
    statusMessage = event.detail;
  }
</script>

<main>
  <div class="panels">
    <ProbePanel
      {connected}
      onconnected={handleConnected}
      ondisconnected={handleDisconnected}
      onstatus={handleStatus}
    />

    {#if connected}
      <FlashPanel onstatus={handleStatus} />
      <RttPanel onstatus={handleStatus} />
    {/if}
  </div>

  <StatusBar message={statusMessage} />
</main>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Display', 'SF Pro Text', 'Helvetica Neue', sans-serif;
    background: #1c1c1e;
    color: #f5f5f7;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .panels {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: 20px;
    gap: 12px;
    min-height: 0;
  }
</style>
