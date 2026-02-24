import { useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";
import { TrackList } from "@/components/track/TrackList";
import { Skeleton } from "@/components/ui/skeleton";
import { useLibraryStore } from "@/stores/libraryStore";
import { useLibrary } from "@/hooks/useLibrary";
import { usePlayback } from "@/hooks/usePlayback";
import { Loader2, Play, Shuffle } from "lucide-react";
import * as tauri from "@/lib/tauri";

export function FavoritesPage() {
  const favorites = useLibraryStore((s) => s.favorites);
  const loading = useLibraryStore((s) => s.loading);
  const loadingMore = useLibraryStore((s) => s.loadingMore);
  const favoritesHasMore = useLibraryStore((s) => s.favoritesHasMore);
  const { loadFavorites, loadMoreFavorites } = useLibrary();
  const { playTracks } = usePlayback();

  useEffect(() => {
    loadFavorites();
  }, [loadFavorites]);

  // Infinite scroll: observe a sentinel element near the bottom
  const sentinelRef = useRef<HTMLDivElement>(null);
  const loadMoreRef = useRef(loadMoreFavorites);
  loadMoreRef.current = loadMoreFavorites;

  const hasMoreRef = useRef(favoritesHasMore);
  hasMoreRef.current = favoritesHasMore;

  const loadingMoreRef = useRef(loadingMore);
  loadingMoreRef.current = loadingMore;

  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasMoreRef.current && !loadingMoreRef.current) {
          loadMoreRef.current();
        }
      },
      { rootMargin: "200px" },
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, []);

  return (
    <div className="flex flex-col gap-6 p-6">
      <div className="flex items-baseline gap-3">
        <h1 className="text-3xl/9 font-bold">Favorites</h1>
        {favorites.length > 0 && (
          <span className="text-sm/5 text-muted-foreground">
            {favorites.length} tracks{favoritesHasMore ? "+" : ""}
          </span>
        )}
      </div>
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
          {/* Sentinel for infinite scroll */}
          <div ref={sentinelRef} className="flex justify-center py-2">
            {loadingMore && (
              <div className="flex items-center gap-2 text-sm/5 text-muted-foreground">
                <Loader2 className="size-4 animate-spin" />
                Loading more...
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
