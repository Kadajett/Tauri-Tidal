import { useEffect, useState, useCallback } from "react";
import { useParams } from "react-router";
import { Play, Shuffle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { AlbumCard } from "@/components/cards/AlbumCard";
import { Skeleton } from "@/components/ui/skeleton";
import { usePlayback } from "@/hooks/usePlayback";
import * as tauri from "@/lib/tauri";
import type { Album, Artist, Track } from "@/types/track";

export function ArtistPage() {
  const { id } = useParams<{ id: string }>();
  const [artist, setArtist] = useState<Artist | null>(null);
  const [albums, setAlbums] = useState<Album[]>([]);
  const [allTracks, setAllTracks] = useState<Track[]>([]);
  const [loading, setLoading] = useState(true);
  const { playTracks } = usePlayback();

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    Promise.all([tauri.getArtist(id), tauri.getArtistAlbums(id)])
      .then(async ([artistData, albumData]) => {
        setArtist(artistData);
        setAlbums(albumData);
        const trackResults = await Promise.all(
          albumData.map((a) => tauri.getAlbumTracks(a.id)),
        );
        setAllTracks(trackResults.flat());
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [id]);

  const handlePlayAll = useCallback(() => {
    if (allTracks.length > 0) {
      playTracks(allTracks, 0);
    }
  }, [allTracks, playTracks]);

  const handleShuffle = useCallback(async () => {
    if (allTracks.length > 0) {
      await playTracks(allTracks, 0);
      await tauri.shuffleQueue();
    }
  }, [allTracks, playTracks]);

  if (loading) {
    return (
      <div className="flex flex-col gap-6 p-6">
        <div className="flex items-center gap-6">
          <Skeleton className="size-40 rounded-full" />
          <Skeleton className="h-10 w-64" />
        </div>
        <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-2">
          {Array.from({ length: 6 }).map((_, i) => (
            <Skeleton key={i} className="aspect-square w-full" />
          ))}
        </div>
      </div>
    );
  }

  if (!artist) return null;

  return (
    <div className="flex flex-col gap-6 p-6">
      <div className="flex items-center gap-6">
        {artist.pictureUrl ? (
          <img
            src={artist.pictureUrl}
            alt={artist.name}
            className="size-40 rounded-full object-cover shadow-sm"
            referrerPolicy="no-referrer"
          />
        ) : (
          <div className="size-40 rounded-full bg-muted" />
        )}
        <h1 className="text-4xl font-bold">{artist.name}</h1>
      </div>

      <div className="flex gap-2">
        <Button
          size="sm"
          onClick={handlePlayAll}
          disabled={allTracks.length === 0}
        >
          <Play className="mr-1 size-4" />
          Play All
        </Button>
        <Button
          size="sm"
          variant="outline"
          onClick={handleShuffle}
          disabled={allTracks.length === 0}
        >
          <Shuffle className="mr-1 size-4" />
          Shuffle
        </Button>
      </div>

      <div>
        <h2 className="mb-4 text-xl/7 font-semibold">Albums</h2>
        <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-2">
          {albums.map((album) => (
            <AlbumCard key={album.id} album={album} />
          ))}
        </div>
      </div>
    </div>
  );
}
