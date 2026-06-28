<script lang="ts">
  import type { Track } from "$lib/data/mockLibrary";

  type Props = {
    track: Track;
  };

  let { track }: Props = $props();
</script>

<footer class="player" aria-label="Now playing">
  <div class="track">
    <div class="cover" style={`--cover-color: ${track.color}`} aria-hidden="true"></div>
    <div class="track-copy">
      <p>{track.title}</p>
      <span>{track.artist}</span>
    </div>
  </div>

  <div class="transport" aria-label="Playback controls">
    <button type="button" aria-label="Previous track">&lt;&lt;</button>
    <button class="play" type="button" aria-label="Pause">||</button>
    <button type="button" aria-label="Next track">&gt;&gt;</button>
  </div>

  <div class="progress-area">
    <span>1:28</span>
    <div class="progress" aria-label="Playback progress">
      <div></div>
    </div>
    <span>{track.duration}</span>
  </div>

  <div class="volume" aria-label="Volume">
    <span>Vol</span>
    <div class="volume-bar">
      <div></div>
    </div>
  </div>
</footer>

<style>
  .player {
    display: grid;
    grid-template-columns: minmax(180px, 1.1fr) auto minmax(220px, 1.6fr) minmax(120px, 0.7fr);
    align-items: center;
    gap: 22px;
    min-height: 86px;
    padding: 14px 24px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    background: #12161c;
  }

  .track {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .cover {
    width: 54px;
    height: 54px;
    flex: 0 0 auto;
    border-radius: 8px;
    background:
      linear-gradient(135deg, color-mix(in srgb, var(--cover-color), white 18%), transparent 58%),
      var(--cover-color);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
  }

  .track-copy {
    min-width: 0;
  }

  .track-copy p,
  .track-copy span {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .track-copy p {
    margin: 0 0 4px;
    color: #f5f7fb;
    font-weight: 750;
  }

  .track-copy span,
  .progress-area span,
  .volume span {
    color: #919ba9;
    font-size: 0.84rem;
    font-weight: 650;
  }

  .transport {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  button {
    display: grid;
    width: 38px;
    height: 38px;
    place-items: center;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #1a2028;
    color: #eef3f8;
    font: inherit;
    font-size: 0.78rem;
    font-weight: 900;
    cursor: default;
  }

  button.play {
    width: 44px;
    height: 44px;
    border-color: #2f8f83;
    background: #2f8f83;
    color: #07110f;
  }

  .progress-area,
  .volume {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .progress,
  .volume-bar {
    height: 7px;
    min-width: 0;
    flex: 1;
    overflow: hidden;
    border-radius: 999px;
    background: #2a313c;
  }

  .progress div,
  .volume-bar div {
    height: 100%;
    border-radius: inherit;
    background: #d8dde4;
  }

  .progress div {
    width: 36%;
  }

  .volume-bar div {
    width: 68%;
    background: #2f8f83;
  }

  @media (max-width: 920px) {
    .player {
      grid-template-columns: minmax(170px, 1fr) auto;
    }

    .progress-area,
    .volume {
      display: none;
    }
  }

  @media (max-width: 560px) {
    .player {
      grid-template-columns: 1fr;
      gap: 12px;
      padding: 14px 16px;
    }

    .transport {
      justify-content: flex-start;
    }
  }
</style>
