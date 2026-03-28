import { defineStore } from "pinia";
import { ref } from "vue";
import type { LibraryRootDto, TrackDto } from "@/api/bindings";
import * as api from "@/api/bindings";

export const useLibraryStore = defineStore("library", () => {
  const roots = ref<LibraryRootDto[]>([]);
  const tracks = ref<TrackDto[]>([]);
  const filter = ref("");
  const loading = ref(false);
  const scanMessage = ref<string | null>(null);

  async function refreshRoots() {
    roots.value = await api.libraryListRoots();
  }

  async function refreshTracks() {
    loading.value = true;
    try {
      tracks.value = await api.libraryListTracks(filter.value || null);
    } finally {
      loading.value = false;
    }
  }

  async function addRoot(path: string, label?: string) {
    await api.libraryAddRoot(path, label ?? null);
    await refreshRoots();
  }

  async function removeRoot(rootId: number) {
    await api.libraryRemoveRoot(rootId);
    await refreshRoots();
    await refreshTracks();
  }

  async function scanRoot(rootId: number) {
    scanMessage.value = null;
    const stats = await api.libraryScanRoot(rootId);
    scanMessage.value = `Scan: ${stats.tracks_upserted} tracks indexed (${stats.files_seen} audio files seen)`;
    await refreshTracks();
  }

  return {
    roots,
    tracks,
    filter,
    loading,
    scanMessage,
    refreshRoots,
    refreshTracks,
    addRoot,
    removeRoot,
    scanRoot,
  };
});
