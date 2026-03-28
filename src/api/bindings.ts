import { invoke } from "@tauri-apps/api/core";

export interface LibraryRootDto {
  id: number;
  path: string;
  label: string | null;
  added_at: number;
}

export interface TrackDto {
  id: number;
  root_id: number;
  path: string;
  title: string | null;
  album: string | null;
  artist: string | null;
  album_artist: string | null;
  track_number: number | null;
  disc_number: number | null;
  duration_ms: number | null;
  sample_rate: number | null;
  channels: number | null;
  bit_depth: number | null;
  codec_hint: string | null;
  container: string | null;
  file_mtime: number;
  file_size: number;
}

export interface ScanStatsDto {
  files_seen: number;
  tracks_upserted: number;
}

export interface PlaybackStateDto {
  path: string | null;
  decoder: string | null;
  duration_ms: number;
  position_ms: number;
  playing: boolean;
  volume: number;
}

export interface TagPatchDto {
  title: string;
  album: string;
  artist: string;
  album_artist: string;
  track_number?: number;
  disc_number?: number;
}

export async function libraryAddRoot(
  path: string,
  label?: string | null,
): Promise<number> {
  return invoke("library_add_root", { path, label: label ?? null });
}

export async function libraryListRoots(): Promise<LibraryRootDto[]> {
  return invoke("library_list_roots");
}

export async function libraryRemoveRoot(rootId: number): Promise<void> {
  return invoke("library_remove_root", { rootId });
}

export async function libraryScanRoot(rootId: number): Promise<ScanStatsDto> {
  return invoke("library_scan_root", { rootId });
}

export async function libraryListTracks(filter: string | null): Promise<TrackDto[]> {
  return invoke("library_list_tracks", { filter });
}

export async function libraryGetTrack(id: number): Promise<TrackDto> {
  return invoke("library_get_track", { id });
}

export async function libraryUpdateTrackTags(
  id: number,
  patch: TagPatchDto,
): Promise<void> {
  return invoke("library_update_track_tags", { id, patch });
}

export async function libraryEmbedCover(id: number, pngBytes: number[]): Promise<void> {
  return invoke("library_embed_cover", { id, pngBytes });
}

export async function playbackLoad(path: string): Promise<void> {
  return invoke("playback_load", { path });
}

export async function playbackPlay(): Promise<void> {
  return invoke("playback_play");
}

export async function playbackPause(): Promise<void> {
  return invoke("playback_pause");
}

export async function playbackSeek(positionMs: number): Promise<void> {
  return invoke("playback_seek", { positionMs });
}

export async function playbackSetVolume(volume: number): Promise<void> {
  return invoke("playback_set_volume", { volume });
}

export async function playbackGetState(): Promise<PlaybackStateDto> {
  return invoke("playback_get_state");
}
