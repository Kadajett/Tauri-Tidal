import { useEffect, useState } from "react";
import { useSearchParams } from "react-router";
import { Button } from "@/components/ui/button";
import { TrackList } from "@/components/track/TrackList";
import { Skeleton } from "@/components/ui/skeleton";
import { usePlayback } from "@/hooks/usePlayback";
import { Play, Shuffle } from "lucide-react";
import * as tauri from "@/lib/tauri";
import type { Track } from "@/types/track";

export function SimilarTracksPage() {
  const [searchParams] = useSearchParams();
  const trackId = searchParams.get("trackId");
  const [tracks, setTracks] = useState<Track[]>([]);
  const [loading, setLoading] = useState(true);
  const { playTracks } = usePlayback();

  useEffect(() => {
    if (!trackId) return;
    setLoading(true);
    tauri
      .getSimilarTracks(trackId)
      .then(setTracks)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [trackId]);

  return (
    <div className="flex flex-col gap-6 p-6">
      <h1 className="text-3xl/9 font-bold">Similar Tracks</h1>
      {loading ? (
        <div className="flex flex-col gap-2">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-12 w-full" />
          ))}
        </div>
      ) : tracks.length === 0 ? (
        <p className="text-muted-foreground">No similar tracks found</p>
      ) : (
        <>
          <div className="flex gap-2">
            <Button size="sm" onClick={() => playTracks(tracks, 0)}>
              <Play className="mr-1 size-4" />
              Play All
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={async () => {
                await playTracks(tracks, 0);
                await tauri.shuffleQueue();
              }}
            >
              <Shuffle className="mr-1 size-4" />
              Shuffle
            </Button>
          </div>
          <TrackList
            tracks={tracks}
            onPlay={(track) => {
              const idx = tracks.findIndex((t) => t.id === track.id);
              playTracks(tracks, Math.max(0, idx));
            }}
          />
        </>
      )}
    </div>
  );
}
