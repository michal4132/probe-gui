<script>
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';

  let { onstatus } = $props();

  let filePath = $state('');
  let flashing = $state(false);

  async function selectFile() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Firmware',
          extensions: ['elf', 'bin', 'hex', 'ihex', 'uf2'],
        }],
      });
      if (selected) {
        filePath = selected;
      }
    } catch (e) {
      onstatus?.({ detail: `File dialog error: ${e}` });
    }
  }

  async function flash() {
    if (!filePath) return;
    flashing = true;
    onstatus?.({ detail: 'Flashing...' });
    try {
      const result = await invoke('flash_firmware', { path: filePath });
      onstatus?.({ detail: result });
    } catch (e) {
      onstatus?.({ detail: `Flash error: ${e}` });
    } finally {
      flashing = false;
    }
  }

  async function resetTarget() {
    try {
      await invoke('reset_target');
      onstatus?.({ detail: 'Target reset' });
    } catch (e) {
      onstatus?.({ detail: `Reset error: ${e}` });
    }
  }
</script>

<section class="panel">
  <h2>Flash / Download</h2>

  <div class="controls">
    <div class="field">
      <label>Firmware File</label>
      <div class="row">
        <input
          type="text"
          bind:value={filePath}
          placeholder="Select firmware file..."
          readonly
        />
        <button class="icon-btn" onclick={selectFile}>📂</button>
      </div>
    </div>

    <div class="actions">
      <button
        class="btn primary"
        onclick={flash}
        disabled={!filePath || flashing}
      >
        {flashing ? 'Flashing...' : 'Flash'}
      </button>
      <button class="btn secondary" onclick={resetTarget}>
        Reset Target
      </button>
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

  .row {
    display: flex;
    gap: 8px;
  }

  input {
    flex: 1;
    padding: 9px 12px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #f5f5f7;
    font-size: 13px;
    font-family: inherit;
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

  .btn.secondary {
    background: rgba(255, 255, 255, 0.08);
    color: #e5e5ea;
    border: none;
  }

  .btn.secondary:hover {
    background: rgba(255, 255, 255, 0.12);
  }
</style>
