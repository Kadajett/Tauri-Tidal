import { useEffect, useState, useCallback } from "react";
import { Shuffle, Repeat, Repeat1, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { TrackList } from "@/components/track/TrackList";
import { useQueueStore } from "@/stores/queueStore";
import * as tauri from "@/lib/tauri";
import { cn } from "@/lib/utils";

export function QueuePage() {
  const { tracks, repeatMode, shuffled } = useQueueStore();
  const setQueue = useQueueStore((s) => s.setQueue);
  const setRepeatMode = useQueueStore((s) => s.setRepeatMode);
  const setShuffled = useQueueStore((s) => s.setShuffled);
  const [loading, setLoading] = useState(true);

  const loadQueue = useCallback(async () => {
    try {
      const q = await tauri.getQueue();
      setQueue(q.tracks, q.currentIndex);
      setRepeatMode(q.repeatMode);
      setShuffled(q.shuffled);
    } catch (err) {
      console.error("Failed to load queue:", err);
    } finally {
      setLoading(false);
    }
  }, [setQueue, setRepeatMode, setShuffled]);

  useEffect(() => {
    loadQueue();
  }, [loadQueue]);

  const handleToggleShuffle = async () => {
    if (shuffled) {
      await tauri.unshuffleQueue();
    } else {
      await tauri.shuffleQueue();
    }
    loadQueue();
  };

  const handleToggleRepeat = async () => {
    const mode = await tauri.toggleRepeat();
    setRepeatMode(mode);
  };

  const handleClear = async () => {
    await tauri.clearQueue();
    loadQueue();
  };

  const RepeatIcon = repeatMode === "one" ? Repeat1 : Repeat;

  return (
    <div className="flex flex-col gap-6 p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl/9 font-bold">Queue</h1>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            className={cn(shuffled && "text-primary")}
            onClick={handleToggleShuffle}
          >
            <Shuffle className="size-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className={cn(repeatMode !== "off" && "text-primary")}
            onClick={handleToggleRepeat}
          >
            <RepeatIcon className="size-4" />
          </Button>
          <Button variant="ghost" size="icon" onClick={handleClear}>
            <Trash2 className="size-4" />
          </Button>
        </div>
      </div>

      {loading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : tracks.length === 0 ? (
        <p className="text-muted-foreground">Queue is empty</p>
      ) : (
        <TrackList
          tracks={tracks}
          onPlay={(track) => {
            const idx = tracks.findIndex((t) => t.id === track.id);
            tauri.playQueueTrack(Math.max(0, idx));
          }}
        />
      )}
    </div>
  );
}
