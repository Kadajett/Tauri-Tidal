import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { TrackList } from "@/components/track/TrackList";
import { AlbumCard } from "@/components/cards/AlbumCard";
import { ArtistCard } from "@/components/cards/ArtistCard";
import { PlaylistCard } from "@/components/cards/PlaylistCard";
import { useSearchStore } from "@/stores/searchStore";
import { Skeleton } from "@/components/ui/skeleton";
import type { Track } from "@/types/track";
import { usePlayback } from "@/hooks/usePlayback";
import { useCallback } from "react";

export function SearchResultsView() {
  const results = useSearchStore((s) => s.results);
  const loading = useSearchStore((s) => s.loading);
  const { playTracks } = usePlayback();

  const handlePlay = useCallback(
    (track: Track) => {
      if (!results) return;
      const index = results.tracks.findIndex((t) => t.id === track.id);
      playTracks(results.tracks, Math.max(0, index));
    },
    [results, playTracks],
  );

  if (loading) {
    return (
      <div className="flex flex-col gap-4 p-6">
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-12 w-full" />
        ))}
      </div>
    );
  }

  if (!results) {
    return null;
  }

  const hasTracks = results.tracks.length > 0;
  const hasAlbums = results.albums.length > 0;
  const hasArtists = results.artists.length > 0;
  const hasPlaylists = results.playlists.length > 0;

  if (!hasTracks && !hasAlbums && !hasArtists && !hasPlaylists) {
    return (
      <div className="flex items-center justify-center py-12 text-muted-foreground">
        No results found
      </div>
    );
  }

  return (
    <Tabs defaultValue="tracks">
      <TabsList>
        {hasTracks && <TabsTrigger value="tracks">Tracks</TabsTrigger>}
        {hasAlbums && <TabsTrigger value="albums">Albums</TabsTrigger>}
        {hasArtists && <TabsTrigger value="artists">Artists</TabsTrigger>}
        {hasPlaylists && (
          <TabsTrigger value="playlists">Playlists</TabsTrigger>
        )}
      </TabsList>

      {hasTracks && (
        <TabsContent value="tracks">
          <TrackList tracks={results.tracks} onPlay={handlePlay} />
        </TabsContent>
      )}

      {hasAlbums && (
        <TabsContent value="albums">
          <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-2">
            {results.albums.map((album) => (
              <AlbumCard key={album.id} album={album} />
            ))}
          </div>
        </TabsContent>
      )}

      {hasArtists && (
        <TabsContent value="artists">
          <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-2">
            {results.artists.map((artist) => (
              <ArtistCard key={artist.id} artist={artist} />
            ))}
          </div>
        </TabsContent>
      )}

      {hasPlaylists && (
        <TabsContent value="playlists">
          <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-2">
            {results.playlists.map((playlist) => (
              <PlaylistCard key={playlist.id} playlist={playlist} />
            ))}
          </div>
        </TabsContent>
      )}
    </Tabs>
  );
}
