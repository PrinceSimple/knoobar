import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { PlaybackStateDto, TrackDto } from "@/api/bindings";
import * as api from "@/api/bindings";

export const usePlayerStore = defineStore("player", () => {
  const state = ref<PlaybackStateDto | null>(null);
  const queue = ref<TrackDto[]>([]);
  const pollHandle = ref<number | null>(null);

  const nowPlaying = computed(() => state.value);

  function startPoll() {
    if (pollHandle.value != null) return;
    pollHandle.value = window.setInterval(async () => {
      try {
        state.value = await api.playbackGetState();
      } catch {
        /* ignore */
      }
    }, 350);
  }

  function stopPoll() {
    if (pollHandle.value != null) {
      clearInterval(pollHandle.value);
      pollHandle.value = null;
    }
  }

  async function refreshState() {
    state.value = await api.playbackGetState();
  }

  async function playTrack(track: TrackDto) {
    queue.value = [track];
    await api.playbackLoad(track.path);
    await api.playbackPlay();
    await refreshState();
    startPoll();
  }

  async function enqueue(track: TrackDto) {
    queue.value = [...queue.value, track];
    if (!state.value?.playing && queue.value.length === 1) {
      await playTrack(track);
    }
  }

  async function togglePlay() {
    if (!state.value?.path) return;
    if (state.value.playing) {
      await api.playbackPause();
    } else {
      await api.playbackPlay();
    }
    await refreshState();
    startPoll();
  }

  async function seek(ms: number) {
    await api.playbackSeek(ms);
    await refreshState();
  }

  async function setVolume(v: number) {
    const clamped = Math.min(1, Math.max(0, v));
    await api.playbackSetVolume(clamped);
    await refreshState();
  }

  return {
    state,
    queue,
    nowPlaying,
    refreshState,
    playTrack,
    enqueue,
    togglePlay,
    seek,
    setVolume,
    startPoll,
    stopPoll,
  };
});
