import { useCallback, useEffect, useState } from "react";
import { SearchBar } from "@/components/search/SearchBar";
import { SearchResultsView } from "@/components/search/SearchResults";
import { TrackList } from "@/components/track/TrackList";
import { Skeleton } from "@/components/ui/skeleton";
import { useSearchStore } from "@/stores/searchStore";
import { usePlayback } from "@/hooks/usePlayback";
import { invoke } from "@tauri-apps/api/core";
import * as tauri from "@/lib/tauri";
import type { RecommendationSection } from "@/types/track";
import type { Track } from "@/types/track";

const headerImageCache = new Map<string, string>();

function useProxiedUrl(src: string | undefined): string | null {
  const [dataUri, setDataUri] = useState<string | null>(
    () => (src && headerImageCache.get(src)) ?? null,
  );

  useEffect(() => {
    if (!src) return;

    const cached = headerImageCache.get(src);
    if (cached) {
      setDataUri(cached);
      return;
    }

    setDataUri(null);
    let cancelled = false;

    invoke<string>("proxy_image", { url: src })
      .then((uri) => {
        if (!cancelled) {
          headerImageCache.set(src, uri);
          setDataUri(uri);
        }
      })
      .catch(() => {});

    return () => {
      cancelled = true;
    };
  }, [src]);

  return dataUri;
}

function SectionSkeleton() {
  return (
    <div className="flex flex-col gap-3">
      <Skeleton className="h-6 w-48" />
      <Skeleton className="h-4 w-32" />
      {Array.from({ length: 4 }).map((_, i) => (
        <Skeleton key={i} className="h-12 w-full" />
      ))}
    </div>
  );
}

interface RecommendationSectionViewProps {
  section: RecommendationSection;
}

function RecommendationSectionView({ section }: RecommendationSectionViewProps) {
  const { playTracks } = usePlayback();
  const [expanded, setExpanded] = useState(false);

  const displayTracks = expanded ? section.tracks : section.tracks.slice(0, 5);

  // Get the first track's artwork at the largest CDN size for the header
  const headerArtworkUrl = section.tracks[0]?.artworkUrl?.replace(
    /\/\d+x\d+\.jpg$/,
    "/1280x1280.jpg",
  );
  const headerDataUri = useProxiedUrl(headerArtworkUrl);

  const handlePlay = useCallback(
    (track: Track) => {
      const idx = section.tracks.findIndex((t) => t.id === track.id);
      playTracks(section.tracks, Math.max(0, idx));
    },
    [section.tracks, playTracks],
  );

  const handlePlayMix = useCallback(() => {
    if (section.tracks.length > 0) {
      playTracks(section.tracks, 0);
    }
  }, [section.tracks, playTracks]);

  return (
    <div className="relative flex flex-col gap-2 overflow-hidden rounded-sm">
      {headerDataUri && (
        <div className="pointer-events-none absolute inset-x-0 top-0 h-20 overflow-hidden">
          <img
            src={headerDataUri}
            alt=""
            className="h-full w-full object-cover object-center"
          />
          <div className="absolute inset-0 bg-linear-to-b from-background/40 to-background" />
        </div>
      )}
      <div className="relative z-10 flex items-center justify-between pt-5 px-3">
        <div>
          <h3 className="text-lg/7 font-semibold">{section.title}</h3>
          {section.subtitle && (
            <p className="text-sm/5 text-muted-foreground">{section.subtitle}</p>
          )}
        </div>
        {section.tracks.length > 0 && (
          <button
            className="rounded-sm bg-primary px-4 py-1.5 text-sm/5 font-medium text-primary-foreground hover:bg-primary/90"
            onClick={handlePlayMix}
          >
            Play Mix
          </button>
        )}
      </div>
      <div className="relative z-10">
        <TrackList tracks={displayTracks} onPlay={handlePlay} />
      </div>
      {section.tracks.length > 5 && (
        <button
          className="relative z-10 self-start px-3 py-1 text-sm/5 text-muted-foreground hover:text-foreground"
          onClick={() => setExpanded((prev) => !prev)}
        >
          {expanded ? "Show less" : `Show all ${section.tracks.length} tracks`}
        </button>
      )}
    </div>
  );
}

export function SearchPage() {
  const query = useSearchStore((s) => s.query);
  const results = useSearchStore((s) => s.results);
  const loading = useSearchStore((s) => s.loading);
  const [sections, setSections] = useState<RecommendationSection[]>([]);
  const [recsLoading, setRecsLoading] = useState(false);

  useEffect(() => {
    setRecsLoading(true);
    tauri
      .getRecommendations()
      .then(setSections)
      .catch(console.error)
      .finally(() => setRecsLoading(false));
  }, []);

  const showSearch = query.length > 0 && (results || loading);

  return (
    <div className="flex flex-col gap-6 p-6">
      <SearchBar />
      {showSearch ? (
        <SearchResultsView />
      ) : (
        <div className="flex flex-col gap-8">
          <h2 className="text-xl/7 font-semibold">Discover</h2>
          {recsLoading ? (
            <div className="flex flex-col gap-8">
              <SectionSkeleton />
              <SectionSkeleton />
            </div>
          ) : sections.length > 0 ? (
            <div className="flex flex-col gap-8">
              {sections.map((section) => (
                <RecommendationSectionView
                  key={section.title}
                  section={section}
                />
              ))}
            </div>
          ) : (
            <p className="py-8 text-center text-sm/5 text-muted-foreground">
              No recommendations available. Try searching for something to get
              started.
            </p>
          )}
        </div>
      )}
    </div>
  );
}
