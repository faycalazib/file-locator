<!--
  SVG filters shared by the theme (CSS: `filter: url(#wobble)`).
  #wobble: slightly shakes the strokes, like a pencil line drawn by hand.
-->
<script lang="ts">
  // Markup only: the filters are referenced from the theme CSS.
</script>

<svg class="defs" aria-hidden="true" focusable="false">
  <filter id="wobble" x="-5%" y="-5%" width="110%" height="110%">
    <feTurbulence type="turbulence" baseFrequency="0.02" numOctaves="2" seed="7" result="noise" />
    <feDisplacementMap in="SourceGraphic" in2="noise" scale="4" />
  </filter>
</svg>

<style>
  .defs {
    position: absolute;
    inline-size: 0;
    block-size: 0;
    overflow: hidden;
  }
</style>
