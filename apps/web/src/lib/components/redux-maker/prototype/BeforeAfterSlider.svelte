<svelte:options runes={false} />
<script>
  // @ts-nocheck
  export let label = 'Visual Preview'
  export let mode = 'comparison'

  let sliderPosition = 50
  let previewEl
  let dragging = false

  function updateFromPoint(clientX) {
    const bounds = previewEl.getBoundingClientRect()
    const next = ((clientX - bounds.left) / bounds.width) * 100
    sliderPosition = Math.max(8, Math.min(92, next))
  }

  function startDrag(event) {
    if (mode !== 'comparison') return
    dragging = true
    updateFromPoint(event.clientX)
    event.currentTarget.setPointerCapture(event.pointerId)
  }

  function drag(event) {
    if (!dragging) return
    updateFromPoint(event.clientX)
  }

  function stopDrag() {
    dragging = false
  }

  function startMouseDrag(event) {
    if (mode !== 'comparison') return
    dragging = true
    updateFromPoint(event.clientX)
  }

  function mouseDrag(event) {
    if (!dragging) return
    updateFromPoint(event.clientX)
  }

  function handleKeydown(event) {
    if (mode !== 'comparison') return
    if (event.key === 'ArrowLeft') {
      event.preventDefault()
      sliderPosition = Math.max(8, sliderPosition - 4)
    }
    if (event.key === 'ArrowRight') {
      event.preventDefault()
      sliderPosition = Math.min(92, sliderPosition + 4)
    }
  }
</script>

<svelte:window on:mousemove={mouseDrag} on:mouseup={stopDrag} />

<div
  bind:this={previewEl}
  class="preview-slider"
  on:pointerdown={startDrag}
  on:pointermove={drag}
  on:pointerup={stopDrag}
  on:pointercancel={stopDrag}
  on:mousedown={startMouseDrag}
  on:keydown={handleKeydown}
  role="slider"
  aria-label={`Before and after preview: ${label}`}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={Math.round(sliderPosition)}
  tabindex="0"
>
  {#if mode === 'comparison'}
    <input
      class="slider-range"
      type="range"
      min="8"
      max="92"
      step="1"
      aria-label={`Move before and after preview: ${label}`}
      bind:value={sliderPosition}
    />
  {/if}

  <div class:tracers-only={mode === 'tracers'} class="preview-layer after-layer">
    <div class="road-grid"></div>
    <div class="horizon"></div>
    <div class="tracer-texture modified"></div>
    <div class="tracer tracer-one"></div>
    <div class="tracer tracer-two"></div>
    <div class="tracer tracer-three"></div>
    <div class="tracer tracer-four"></div>
    <div class="impact-glow"></div>
  </div>

  {#if mode === 'comparison'}
    <div class="preview-layer before-layer" style={`clip-path: inset(0 ${100 - sliderPosition}% 0 0)`}>
      <div class="road-grid"></div>
      <div class="horizon"></div>
      <div class="tracer-texture source"></div>
      <div class="old-noise"></div>
    </div>

    <span class="preview-label before-label">Before</span>
    <span class="preview-label after-label">After</span>

    <div class="comparison-handle" style={`left: ${sliderPosition}%`}>
      <span class="handle-line"></span>
      <span class="handle-knob">
        <svg class="icon" width="18" height="18" viewBox="0 0 24 24"><path d="M8 6 4 12l4 6M16 6l4 6-4 6" /></svg>
      </span>
    </div>
  {:else}
    <span class="preview-label generated-label">AI tracers</span>
  {/if}
</div>

<style>
  .slider-range {
    position: absolute;
    inset: 0;
    z-index: 7;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: ew-resize;
  }

  .slider-range:focus {
    outline: none;
  }
</style>
