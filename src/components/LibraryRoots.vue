<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useLibraryStore } from "@/stores/library";

const lib = useLibraryStore();
const newPath = ref("");
const newLabel = ref("");

onMounted(() => {
  void lib.refreshRoots();
});

async function add() {
  const p = newPath.value.trim();
  if (!p) return;
  const l = newLabel.value.trim() || null;
  await lib.addRoot(p, l ?? undefined);
  newPath.value = "";
  newLabel.value = "";
}
</script>

<template>
  <section class="rounded-lg border border-surface-3 bg-surface-1 p-4">
    <h2 class="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-400">Library roots</h2>
    <p class="mb-3 text-xs text-slate-500">
      Add a folder on a mapped drive or UNC path (for example <code class="rounded bg-surface-2 px-1">\\server\music</code>).
      The share must be reachable to Windows before scanning.
    </p>
    <div class="flex flex-col gap-2 sm:flex-row sm:items-end">
      <label class="flex flex-1 flex-col gap-1 text-xs text-slate-400">
        Folder path
        <input
          v-model="newPath"
          class="rounded-md border border-surface-3 bg-surface-0 px-2 py-1.5 text-sm text-slate-100 outline-none focus:border-accent"
          placeholder="Z:\Music or \\NAS\media\flac"
          type="text"
        />
      </label>
      <label class="flex w-full flex-col gap-1 text-xs text-slate-400 sm:w-40">
        Label (optional)
        <input
          v-model="newLabel"
          class="rounded-md border border-surface-3 bg-surface-0 px-2 py-1.5 text-sm text-slate-100 outline-none focus:border-accent"
          type="text"
        />
      </label>
      <button
        type="button"
        class="rounded-md bg-accent px-3 py-2 text-sm font-medium text-surface-0 hover:bg-accent-muted"
        @click="add"
      >
        Add root
      </button>
    </div>
    <ul class="mt-4 divide-y divide-surface-3 border-t border-surface-3 pt-3">
      <li v-for="r in lib.roots" :key="r.id" class="flex flex-wrap items-center justify-between gap-2 py-2 text-sm">
        <div>
          <div class="font-medium text-slate-100">{{ r.label || r.path }}</div>
          <div class="text-xs text-slate-500">{{ r.path }}</div>
        </div>
        <div class="flex gap-2">
          <button
            type="button"
            class="rounded border border-surface-3 px-2 py-1 text-xs hover:border-accent hover:text-accent"
            @click="lib.scanRoot(r.id)"
          >
            Scan
          </button>
          <button
            type="button"
            class="rounded border border-red-900/60 px-2 py-1 text-xs text-red-300 hover:bg-red-950/40"
            @click="lib.removeRoot(r.id)"
          >
            Remove
          </button>
        </div>
      </li>
      <li v-if="!lib.roots.length" class="py-3 text-sm text-slate-500">No roots yet.</li>
    </ul>
    <p v-if="lib.scanMessage" class="mt-3 text-xs text-accent">{{ lib.scanMessage }}</p>
  </section>
</template>
