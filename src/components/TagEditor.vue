<script setup lang="ts">
import { computed, reactive, watch } from "vue";
import type { TrackDto, TagPatchDto } from "@/api/bindings";
import * as api from "@/api/bindings";

const props = defineProps<{ track: TrackDto | null }>();
const emit = defineEmits<{ close: []; saved: [] }>();

const form = reactive({
  title: "",
  album: "",
  artist: "",
  album_artist: "",
  track_number: "" as string,
  disc_number: "" as string,
});

const open = computed(() => props.track != null);

watch(
  () => props.track,
  (t) => {
    if (!t) return;
    form.title = t.title ?? "";
    form.album = t.album ?? "";
    form.artist = t.artist ?? "";
    form.album_artist = t.album_artist ?? "";
    form.track_number = t.track_number != null ? String(t.track_number) : "";
    form.disc_number = t.disc_number != null ? String(t.disc_number) : "";
  },
  { immediate: true },
);

function close() {
  emit("close");
}

async function save() {
  if (!props.track) return;
  const patch: TagPatchDto = {
    title: form.title,
    album: form.album,
    artist: form.artist,
    album_artist: form.album_artist,
    track_number: form.track_number ? Number(form.track_number) : undefined,
    disc_number: form.disc_number ? Number(form.disc_number) : undefined,
  };
  await api.libraryUpdateTrackTags(props.track.id, patch);
  emit("saved");
  close();
}

async function onCoverPicked(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file || !props.track) return;
  const buf = new Uint8Array(await file.arrayBuffer());
  await api.libraryEmbedCover(props.track.id, Array.from(buf));
  emit("saved");
  close();
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      @click.self="close"
    >
      <div class="w-full max-w-md rounded-lg border border-surface-3 bg-surface-1 p-5 shadow-xl">
        <h3 class="mb-4 text-lg font-semibold text-slate-100">Edit tags</h3>
        <p class="mb-4 truncate text-xs text-slate-500">{{ track?.path }}</p>
        <div class="space-y-3 text-sm">
          <label class="block text-xs text-slate-400">
            Title
            <input v-model="form.title" class="mt-1 w-full rounded border border-surface-3 bg-surface-0 px-2 py-1" />
          </label>
          <label class="block text-xs text-slate-400">
            Artist
            <input v-model="form.artist" class="mt-1 w-full rounded border border-surface-3 bg-surface-0 px-2 py-1" />
          </label>
          <label class="block text-xs text-slate-400">
            Album
            <input v-model="form.album" class="mt-1 w-full rounded border border-surface-3 bg-surface-0 px-2 py-1" />
          </label>
          <label class="block text-xs text-slate-400">
            Album artist
            <input v-model="form.album_artist" class="mt-1 w-full rounded border border-surface-3 bg-surface-0 px-2 py-1" />
          </label>
          <div class="flex gap-2">
            <label class="block flex-1 text-xs text-slate-400">
              Track #
              <input v-model="form.track_number" class="mt-1 w-full rounded border border-surface-3 bg-surface-0 px-2 py-1" />
            </label>
            <label class="block flex-1 text-xs text-slate-400">
              Disc #
              <input v-model="form.disc_number" class="mt-1 w-full rounded border border-surface-3 bg-surface-0 px-2 py-1" />
            </label>
          </div>
          <label class="block text-xs text-slate-400">
            Embed cover (PNG/JPEG file)
            <input type="file" accept="image/*" class="mt-1 w-full text-xs" @change="onCoverPicked" />
          </label>
        </div>
        <div class="mt-6 flex justify-end gap-2">
          <button type="button" class="rounded border border-surface-3 px-3 py-1.5 text-sm" @click="close">
            Cancel
          </button>
          <button
            type="button"
            class="rounded bg-accent px-3 py-1.5 text-sm font-medium text-surface-0"
            @click="save"
          >
            Save to file
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
