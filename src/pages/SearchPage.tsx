import { useCallback, useEffect, useState } from "react";
import { SearchBar } from "@/components/search/SearchBar";
import { SearchResultsView } from "@/components/search/SearchResults";
import { TrackList } from "@/components/track/TrackList";
import { Skeleton } from "@/components/ui/skeleton";
import { useSearchStore } from "@/stores/searchStore";
import { usePlayback } from "@/hooks/usePlayback";
import * as tauri from "@/lib/tauri";
import type { Track } from "@/types/track";

export function SearchPage() {
  const query = useSearchStore((s) => s.query);
  const results = useSearchStore((s) => s.results);
  const loading = useSearchStore((s) => s.loading);
  const [recommendations, setRecommendations] = useState<Track[]>([]);
  const [recsLoading, setRecsLoading] = useState(false);
  const { playTracks } = usePlayback();

  useEffect(() => {
    setRecsLoading(true);
    tauri.getRecommendations()
      .then(setRecommendations)
      .catch(console.error)
      .finally(() => setRecsLoading(false));
  }, []);

  const handleRecPlay = useCallback(
    (track: Track) => {
      const idx = recommendations.findIndex((t) => t.id === track.id);
      playTracks(recommendations, Math.max(0, idx));
    },
    [recommendations, playTracks],
  );

  const showSearch = query.length > 0 && (results || loading);

  return (
    <div className="flex flex-col gap-6 p-6">
      <SearchBar />
      {showSearch ? (
        <SearchResultsView />
      ) : (
        <div>
          <h2 className="mb-4 text-xl/7 font-semibold">Recommended</h2>
          {recsLoading ? (
            <div className="flex flex-col gap-2">
              {Array.from({ length: 5 }).map((_, i) => (
                <Skeleton key={i} className="h-12 w-full" />
              ))}
            </div>
          ) : recommendations.length > 0 ? (
            <TrackList tracks={recommendations} onPlay={handleRecPlay} />
          ) : (
            <p className="py-8 text-center text-sm/5 text-muted-foreground">
              No recommendations available
            </p>
          )}
        </div>
      )}
    </div>
  );
}
