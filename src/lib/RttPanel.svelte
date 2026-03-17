<script>
  import { invoke } from '@tauri-apps/api/core';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import '@xterm/xterm/css/xterm.css';

  let { onstatus } = $props();

  let rttConnected = $state(false);
  let channels = $state([]);
  let scanRegion = $state('');
  let polling = $state(false);
  let pollInterval = null;
  let terminal = null;
  let fitAddon = null;
  let terminalEl = $state(null);
  let resizeObserver = null;

  function initTerminal() {
    if (terminal) terminal.dispose();

    terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: 'block',
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Menlo', monospace",
      fontSize: 13,
      lineHeight: 1.3,
      scrollback: 10000,
      convertEol: false,
      theme: {
        background: '#1c1c1e',
        foreground: '#e5e5ea',
        cursor: '#0a84ff',
        cursorAccent: '#1c1c1e',
        selectionBackground: 'rgba(10, 132, 255, 0.3)',
        selectionForeground: '#ffffff',
        black: '#3a3a3c',
        red: '#ff453a',
        green: '#30d158',
        yellow: '#ffd60a',
        blue: '#0a84ff',
        magenta: '#bf5af2',
        cyan: '#64d2ff',
        white: '#e5e5ea',
        brightBlack: '#636366',
        brightRed: '#ff6961',
        brightGreen: '#4dd964',
        brightYellow: '#ffe620',
        brightBlue: '#409cff',
        brightMagenta: '#da8fff',
        brightCyan: '#70d7ff',
        brightWhite: '#f5f5f7',
      },
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(terminalEl);
    fitAddon.fit();

    // Send keystrokes to the target (including Tab, arrows, etc.)
    terminal.onData((data) => {
      invoke('rtt_write', { data }).catch((e) => {
        onstatus?.({ detail: `RTT write error: ${e}` });
      });
    });

    // Resize terminal when container resizes
    resizeObserver = new ResizeObserver(() => {
      fitAddon?.fit();
    });
    resizeObserver.observe(terminalEl);
  }

  function destroyTerminal() {
    resizeObserver?.disconnect();
    resizeObserver = null;
    terminal?.dispose();
    terminal = null;
    fitAddon = null;
  }

  async function attachRtt() {
    try {
      const region = scanRegion || undefined;
      channels = await invoke('rtt_attach', { scanRegion: region });
      rttConnected = true;
      onstatus?.({ detail: `RTT attached (${channels.length} channels)` });

      // Wait a tick for the DOM to render, then init terminal
      await new Promise((r) => setTimeout(r, 0));
      initTerminal();
      startPolling();
    } catch (e) {
      onstatus?.({ detail: `RTT attach error: ${e}` });
    }
  }

  async function detachRtt() {
    stopPolling();
    destroyTerminal();
    try {
      await invoke('rtt_detach');
      rttConnected = false;
      channels = [];
      onstatus?.({ detail: 'RTT detached' });
    } catch (e) {
      onstatus?.({ detail: `RTT detach error: ${e}` });
    }
  }

  function startPolling() {
    if (polling) return;
    polling = true;
    pollInterval = setInterval(pollRtt, 50);
  }

  function stopPolling() {
    polling = false;
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }

  async function pollRtt() {
    try {
      const data = await invoke('rtt_read');
      if (data) {
        terminal?.write(data);
      }
    } catch (e) {
      stopPolling();
      onstatus?.({ detail: `RTT read error: ${e}` });
    }
  }

  function clearTerminal() {
    terminal?.clear();
  }
</script>

<section class="panel" class:panel-grow={rttConnected}>
  <h2>RTT Terminal</h2>

  {#if !rttConnected}
    <div class="controls">
      <div class="field">
        <label for="scan-region">Scan Region <span class="hint">(optional, e.g. 0x20000000..0x20010000)</span></label>
        <input id="scan-region" type="text" bind:value={scanRegion} placeholder="Auto-detect" />
      </div>
      <button class="btn primary" onclick={attachRtt}>Attach RTT</button>
    </div>
  {:else}
    <div class="rtt-container">
      <div class="rtt-toolbar">
        <span class="channel-info">
          {#each channels as ch}
            <span class="channel-badge">CH{ch.number}: {ch.name || 'unnamed'} ({ch.buffer_size}B)</span>
          {/each}
        </span>
        <div class="rtt-actions">
          <button class="icon-btn" onclick={clearTerminal} title="Clear">🗑</button>
          <button class="btn danger small" onclick={detachRtt}>Detach</button>
        </div>
      </div>

      <div class="terminal-wrapper" bind:this={terminalEl}></div>
    </div>
  {/if}
</section>

<style>
  .panel {
    background: #2c2c2e;
    border-radius: 12px;
    padding: 20px;
    border: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    flex-direction: column;
  }

  .panel-grow {
    flex: 1;
    min-height: max(200px, 50vh);
    overflow: hidden;
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

  input {
    padding: 9px 12px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #f5f5f7;
    font-size: 13px;
    font-family: inherit;
    transition: border-color 0.15s;
  }

  input:focus {
    outline: none;
    border-color: #0a84ff;
  }

  .rtt-container {
    display: flex;
    flex-direction: column;
    gap: 10px;
    flex: 1;
    min-height: 0;
  }

  .rtt-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  .channel-info {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .channel-badge {
    font-size: 11px;
    padding: 3px 10px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    color: #98989d;
  }

  .rtt-actions {
    display: flex;
    gap: 6px;
  }

  .terminal-wrapper {
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    overflow: hidden;
    flex: 1;
    min-height: 0;
  }

  .terminal-wrapper :global(.xterm) {
    padding: 10px;
    height: 100%;
  }

  .terminal-wrapper :global(.xterm-viewport) {
    background-color: #1c1c1e !important;
  }

  .terminal-wrapper :global(.xterm-viewport::-webkit-scrollbar) {
    width: 8px;
  }

  .terminal-wrapper :global(.xterm-viewport::-webkit-scrollbar-track) {
    background: transparent;
  }

  .terminal-wrapper :global(.xterm-viewport::-webkit-scrollbar-thumb) {
    background: rgba(255, 255, 255, 0.15);
    border-radius: 4px;
  }

  .terminal-wrapper :global(.xterm-viewport::-webkit-scrollbar-thumb:hover) {
    background: rgba(255, 255, 255, 0.25);
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

  .btn.small {
    padding: 5px 14px;
    font-size: 12px;
  }

  .btn.primary {
    background: #0a84ff;
    color: white;
  }

  .btn.primary:hover {
    background: #409cff;
  }

  .btn.danger {
    background: rgba(255, 69, 58, 0.15);
    color: #ff453a;
    border: none;
  }

  .btn.danger:hover {
    background: rgba(255, 69, 58, 0.25);
  }

  .icon-btn {
    padding: 5px 10px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    color: #e5e5ea;
    cursor: pointer;
    transition: background 0.15s;
  }

  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.1);
  }
</style>
