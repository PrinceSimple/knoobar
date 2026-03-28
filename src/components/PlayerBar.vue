<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { usePlayerStore } from "@/stores/player";

const player = usePlayerStore();
const scrubber = ref(0);
const dragging = ref(false);

const duration = computed(() => player.state?.duration_ms ?? 0);
const position = computed(() => player.state?.position_ms ?? 0);

onMounted(() => {
  void player.refreshState();
  player.startPoll();
});

watch(
  () => player.state?.position_ms,
  (p) => {
    if (dragging.value) return;
    scrubber.value = p ?? 0;
  },
  { immediate: true },
);

const decoderLabel = computed(() => {
  const d = player.state?.decoder;
  if (!d) return "";
  return d === "ffmpeg" ? "FFmpeg" : "Symphonia";
});

async function onScrubCommit() {
  dragging.value = false;
  await player.seek(Math.round(scrubber.value));
}

async function onVolumeInput(e: Event) {
  const v = Number((e.target as HTMLInputElement).value);
  await player.setVolume(v);
}
</script>

<template>
  <footer class="border-t border-surface-3 bg-surface-1 px-4 py-3">
    <div class="mx-auto flex max-w-6xl flex-col gap-3 sm:flex-row sm:items-center">
      <div class="min-w-0 flex-1">
        <div class="truncate text-sm font-medium text-slate-100">
          {{ player.state?.path?.split("\\").pop()?.split("/").pop() || "Nothing loaded" }}
        </div>
        <div class="flex flex-wrap items-center gap-2 text-xs text-slate-500">
          <span v-if="decoderLabel" class="rounded bg-surface-2 px-1.5 py-0.5 text-[10px] uppercase tracking-wide">
            {{ decoderLabel }}
          </span>
          <span>{{ player.state?.playing ? "Playing" : "Paused" }}</span>
        </div>
      </div>
      <div class="flex flex-[2] flex-col gap-1">
        <input
          v-model.number="scrubber"
          type="range"
          :max="Math.max(duration, 1)"
          min="0"
          step="250"
          class="w-full accent-accent"
          @pointerdown="dragging = true"
          @change="onScrubCommit"
        />
        <div class="flex justify-between text-[11px] text-slate-500">
          <span>{{ Math.floor(position / 60000) }}:{{ Math.floor((position / 1000) % 60).toString().padStart(2, "0") }}</span>
          <span>{{ Math.floor(duration / 60000) }}:{{ Math.floor((duration / 1000) % 60).toString().padStart(2, "0") }}</span>
        </div>
      </div>
      <div class="flex items-center gap-3 sm:w-64">
        <button
          type="button"
          class="rounded-md bg-accent px-4 py-2 text-sm font-semibold text-surface-0 disabled:opacity-40"
          :disabled="!player.state?.path"
          @click="player.togglePlay()"
        >
          {{ player.state?.playing ? "Pause" : "Play" }}
        </button>
        <label class="flex flex-1 items-center gap-2 text-xs text-slate-400">
          Vol
          <input
            :value="player.state?.volume ?? 1"
            type="range"
            min="0"
            max="1"
            step="0.02"
            class="w-full accent-accent"
            @input="onVolumeInput"
          />
        </label>
      </div>
    </div>
  </footer>
</template>
