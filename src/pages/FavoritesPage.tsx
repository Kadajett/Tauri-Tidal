import { useEffect } from "react";
import { Button } from "@/components/ui/button";
import { TrackList } from "@/components/track/TrackList";
import { Skeleton } from "@/components/ui/skeleton";
import { useLibraryStore } from "@/stores/libraryStore";
import { useLibrary } from "@/hooks/useLibrary";
import { usePlayback } from "@/hooks/usePlayback";
import { Play, Shuffle } from "lucide-react";
import * as tauri from "@/lib/tauri";

export function FavoritesPage() {
  const favorites = useLibraryStore((s) => s.favorites);
  const loading = useLibraryStore((s) => s.loading);
  const { loadFavorites } = useLibrary();
  const { playTracks } = usePlayback();

  useEffect(() => {
    loadFavorites();
  }, [loadFavorites]);

  return (
    <div className="flex flex-col gap-6 p-6">
      <h1 className="text-3xl/9 font-bold">Favorites</h1>
      {loading ? (
        <div className="flex flex-col gap-2">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-12 w-full" />
          ))}
        </div>
      ) : favorites.length === 0 ? (
        <p className="text-muted-foreground">
          No favorites yet. Heart a track to add it here.
        </p>
      ) : (
        <>
          <div className="flex gap-2">
            <Button
              size="sm"
              onClick={() => playTracks(favorites, 0)}
            >
              <Play className="mr-1 size-4" />
              Play All
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={async () => {
                await playTracks(favorites, 0);
                await tauri.shuffleQueue();
              }}
            >
              <Shuffle className="mr-1 size-4" />
              Shuffle
            </Button>
          </div>
          <TrackList
            tracks={favorites}
            onPlay={(track) => {
              const idx = favorites.findIndex((t) => t.id === track.id);
              playTracks(favorites, Math.max(0, idx));
            }}
          />
        </>
      )}
    </div>
  );
}
