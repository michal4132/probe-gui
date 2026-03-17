<script>
  let { message } = $props();
</script>

<footer>
  <span class="dot" class:connected={message !== 'Disconnected'}></span>
  {message}
</footer>

<style>
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 24px;
    background: #2c2c2e;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    font-size: 12px;
    color: #98989d;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #48484a;
    transition: background 0.2s;
  }

  .dot.connected {
    background: #30d158;
  }
</style>
