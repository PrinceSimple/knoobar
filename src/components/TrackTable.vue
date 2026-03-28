<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useLibraryStore } from "@/stores/library";
import { usePlayerStore } from "@/stores/player";
import type { TrackDto } from "@/api/bindings";

const lib = useLibraryStore();
const player = usePlayerStore();

const emit = defineEmits<{ edit: [track: TrackDto] }>();

onMounted(() => {
  void lib.refreshTracks();
});

watch(
  () => lib.filter,
  () => {
    void lib.refreshTracks();
  },
);

function formatDuration(ms: number | null) {
  if (ms == null || ms <= 0) return "—";
  const s = Math.round(ms / 1000);
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${m}:${r.toString().padStart(2, "0")}`;
}

async function onRowDblClick(t: TrackDto) {
  await player.playTrack(t);
}

function onEdit(t: TrackDto) {
  emit("edit", t);
}
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col rounded-lg border border-surface-3 bg-surface-1">
    <div class="flex items-center gap-2 border-b border-surface-3 px-3 py-2">
      <label class="flex flex-1 items-center gap-2 text-xs text-slate-400">
        Filter
        <input
          v-model="lib.filter"
          class="flex-1 rounded-md border border-surface-3 bg-surface-0 px-2 py-1 text-sm text-slate-100 outline-none focus:border-accent"
          placeholder="title, album, artist…"
          type="search"
        />
      </label>
      <button
        type="button"
        class="rounded border border-surface-3 px-2 py-1 text-xs hover:border-accent"
        @click="lib.refreshTracks()"
      >
        Refresh
      </button>
    </div>
    <div class="min-h-0 flex-1 overflow-auto">
      <table class="w-full border-collapse text-left text-sm">
        <thead class="sticky top-0 bg-surface-2 text-xs uppercase text-slate-400">
          <tr>
            <th class="px-3 py-2 font-medium">Title</th>
            <th class="px-3 py-2 font-medium">Artist</th>
            <th class="px-3 py-2 font-medium">Album</th>
            <th class="px-3 py-2 font-medium">Time</th>
            <th class="px-3 py-2 font-medium">Fmt</th>
            <th class="px-3 py-2 font-medium"></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="t in lib.tracks"
            :key="t.id"
            class="cursor-pointer border-t border-surface-3 hover:bg-surface-2/80"
            @dblclick="onRowDblClick(t)"
          >
            <td class="max-w-[220px] truncate px-3 py-2 text-slate-100">
              {{ t.title || "—" }}
            </td>
            <td class="max-w-[160px] truncate px-3 py-2 text-slate-300">
              {{ t.artist || "—" }}
            </td>
            <td class="max-w-[160px] truncate px-3 py-2 text-slate-300">
              {{ t.album || "—" }}
            </td>
            <td class="whitespace-nowrap px-3 py-2 text-slate-400">
              {{ formatDuration(t.duration_ms) }}
            </td>
            <td class="whitespace-nowrap px-3 py-2 text-xs text-slate-500">
              {{ t.codec_hint || "—" }}
            </td>
            <td class="px-3 py-2 text-right">
              <button
                type="button"
                class="rounded border border-surface-3 px-2 py-0.5 text-xs hover:border-accent"
                @click.stop="onEdit(t)"
              >
                Tags
              </button>
            </td>
          </tr>
          <tr v-if="!lib.tracks.length && !lib.loading">
            <td colspan="6" class="px-3 py-6 text-center text-slate-500">No tracks. Add a root and scan.</td>
          </tr>
          <tr v-if="lib.loading">
            <td colspan="6" class="px-3 py-6 text-center text-slate-500">Loading…</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
