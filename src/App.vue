<script setup lang="ts">
import { ref } from "vue";
import LibraryRoots from "./components/LibraryRoots.vue";
import TrackTable from "./components/TrackTable.vue";
import PlayerBar from "./components/PlayerBar.vue";
import TagEditor from "./components/TagEditor.vue";
import type { TrackDto } from "@/api/bindings";
import { useLibraryStore } from "@/stores/library";

const lib = useLibraryStore();
const editing = ref<TrackDto | null>(null);

function onEdit(t: TrackDto) {
  editing.value = t;
}

function onTagSaved() {
  void lib.refreshTracks();
}
</script>

<template>
  <div class="flex h-full flex-col">
    <header class="border-b border-surface-3 bg-surface-1 px-4 py-3">
      <div class="mx-auto flex max-w-6xl items-baseline justify-between gap-4">
        <div>
          <h1 class="text-xl font-semibold tracking-tight text-slate-50">knoobar</h1>
          <p class="text-xs text-slate-500">Local library • Symphonia + optional FFmpeg in PATH</p>
        </div>
      </div>
    </header>
    <main class="mx-auto flex min-h-0 w-full max-w-6xl flex-1 flex-col gap-4 overflow-hidden p-4">
      <LibraryRoots />
      <TrackTable class="min-h-0" @edit="onEdit" />
    </main>
    <PlayerBar />
    <TagEditor :track="editing" @close="editing = null" @saved="onTagSaved" />
  </div>
</template>
