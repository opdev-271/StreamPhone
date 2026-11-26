<script lang="ts">
  let now = $state(new Date());
  $effect(() => {
    const t = setInterval(() => (now = new Date()), 1000);
    return () => clearInterval(t);
  });
  const time = $derived(now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }));
</script>

<!-- Shown while the PC is unreachable: the phone still works as a clock. -->
<div class="offline">
  <div class="time">{time}</div>
  <div class="hint"><span class="pulse"></span>Čekám na počítač…</div>
</div>

<style>
  .offline {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    background: var(--bg);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    z-index: 10;
  }
  .time {
    font-size: 34vmin;
    font-weight: 500;
    line-height: 1;
    letter-spacing: -0.045em;
  }
  .hint {
    margin-top: 5vmin;
    color: var(--mute);
    display: flex;
    align-items: center;
  }
  .pulse {
    width: 1.6vmin;
    height: 1.6vmin;
    border-radius: 50%;
    background: var(--warn);
    margin-right: 1.6vmin;
    animation: pulse 1.6s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.25;
    }
  }
</style>
