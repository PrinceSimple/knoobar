# FFmpeg on Windows (WMA, DSD/DSF, DFF)

knoobar decodes most formats with **Symphonia**. For **WMA** and **DSD** containers (**DSF**, **DFF**), playback uses an **FFmpeg** subprocess (`ffmpeg` must be on your `PATH`).

1. Install a recent [FFmpeg](https://ffmpeg.org/download.html) build for Windows (e.g. gyan.dev or BtbN full builds).
2. Add the folder that contains `ffmpeg.exe` and `ffprobe.exe` to your user or system **PATH**.
3. Restart the app after changing PATH.

If FFmpeg is missing, those formats will not play. Optional: install **ffprobe** as well; it improves duration metadata during library scans for exotic files.
