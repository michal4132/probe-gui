<script>
  import { invoke } from '@tauri-apps/api/core';

  let { connected, onconnected, ondisconnected, onstatus } = $props();

  let probes = $state([]);
  let chips = $state([]);
  let chipFilter = $state('');
  let selectedProbe = $state(0);
  let selectedChip = $state('');
  let loading = $state(false);

  async function refreshProbes() {
    try {
      probes = await invoke('list_probes');
      onstatus?.({ detail: `Found ${probes.length} probe(s)` });
    } catch (e) {
      onstatus?.({ detail: `Error listing probes: ${e}` });
    }
  }

  async function loadChips() {
    if (chips.length > 0) return;
    try {
      chips = await invoke('list_chips');
    } catch (e) {
      onstatus?.({ detail: `Error loading chips: ${e}` });
    }
  }

  async function connect() {
    loading = true;
    try {
      const chip = selectedChip || null;
      const target = await invoke('connect', {
        probeIndex: selectedProbe,
        chip,
      });
      onconnected?.({ detail: target });
    } catch (e) {
      onstatus?.({ detail: `Connection failed: ${e}` });
    } finally {
      loading = false;
    }
  }

  async function disconnect() {
    try {
      await invoke('disconnect');
      ondisconnected?.();
    } catch (e) {
      onstatus?.({ detail: `Disconnect error: ${e}` });
    }
  }

  let filteredChips = $derived(
    chipFilter.length >= 2
      ? chips.filter(c => c.toLowerCase().includes(chipFilter.toLowerCase()))
      : []
  );

  $effect(() => {
    refreshProbes();
    loadChips();
  });
</script>

<section class="panel">
  <h2>Probe & Target</h2>

  <div class="controls">
    <div class="field">
      <label>Debug Probe</label>
      <div class="row">
        <select bind:value={selectedProbe} disabled={connected}>
          {#each probes as probe}
            <option value={probe.index}>
              {probe.identifier}
              {probe.serial_number ? `(${probe.serial_number})` : ''}
            </option>
          {:else}
            <option value={0}>No probes found</option>
          {/each}
        </select>
        <button class="icon-btn" onclick={refreshProbes} title="Refresh probes">↻</button>
      </div>
    </div>

    <div class="field">
      <label>Target Chip <span class="hint">(leave empty to auto-detect)</span></label>
      <input
        type="text"
        bind:value={chipFilter}
        placeholder="Search chips..."
        disabled={connected}
      />
      {#if filteredChips.length > 0 && !connected}
        <div class="chip-list">
          {#each filteredChips.slice(0, 50) as chip}
            <button
              class="chip-option"
              class:selected={selectedChip === chip}
              onclick={() => { selectedChip = chip; chipFilter = chip; }}
            >
              {chip}
            </button>
          {/each}
          {#if filteredChips.length > 50}
            <span class="hint">...and {filteredChips.length - 50} more</span>
          {/if}
        </div>
      {/if}
    </div>

    <div class="actions">
      {#if connected}
        <button class="btn danger" onclick={disconnect}>Disconnect</button>
      {:else}
        <button
          class="btn primary"
          onclick={connect}
          disabled={probes.length === 0 || loading}
        >
          {loading ? 'Connecting...' : 'Connect'}
        </button>
      {/if}
    </div>
  </div>
</section>

<style>
  .panel {
    background: #2c2c2e;
    border-radius: 12px;
    padding: 20px;
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  h2 {
    margin: 0 0 16px;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: #98989d;
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    font-size: 13px;
    font-weight: 500;
    color: #f5f5f7;
  }

  .hint {
    color: #636366;
    font-weight: 400;
    font-style: normal;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  select, input {
    flex: 1;
    padding: 9px 12px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #f5f5f7;
    font-size: 13px;
    font-family: inherit;
    transition: border-color 0.15s;
  }

  select:focus, input:focus {
    outline: none;
    border-color: #0a84ff;
  }

  .icon-btn {
    padding: 9px 12px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #f5f5f7;
    cursor: pointer;
    font-size: 15px;
    transition: background 0.15s;
  }

  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .chip-list {
    max-height: 150px;
    overflow-y: auto;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 8px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
  }

  .chip-option {
    padding: 5px 10px;
    background: rgba(255, 255, 255, 0.08);
    border: none;
    border-radius: 6px;
    color: #e5e5ea;
    cursor: pointer;
    font-size: 12px;
    font-family: inherit;
    transition: background 0.15s;
  }

  .chip-option:hover {
    background: rgba(255, 255, 255, 0.14);
  }

  .chip-option.selected {
    background: #0a84ff;
    color: white;
  }

  .actions {
    display: flex;
    gap: 8px;
    padding-top: 4px;
  }

  .btn {
    padding: 9px 22px;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    font-family: inherit;
    transition: opacity 0.15s;
  }

  .btn:active {
    opacity: 0.7;
  }

  .btn.primary {
    background: #0a84ff;
    color: white;
  }

  .btn.primary:hover {
    background: #409cff;
  }

  .btn.primary:disabled {
    background: rgba(255, 255, 255, 0.08);
    color: #48484a;
    cursor: not-allowed;
    opacity: 1;
  }

  .btn.danger {
    background: rgba(255, 69, 58, 0.15);
    color: #ff453a;
    border: none;
  }

  .btn.danger:hover {
    background: rgba(255, 69, 58, 0.25);
  }
</style>
