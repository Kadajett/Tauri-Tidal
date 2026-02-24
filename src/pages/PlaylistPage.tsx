import { useEffect, useState } from "react";
import { useParams } from "react-router";
import { Button } from "@/components/ui/button";
import { ProxiedImage } from "@/components/ui/proxied-image";
import { TrackList } from "@/components/track/TrackList";
import { Skeleton } from "@/components/ui/skeleton";
import { usePlayback } from "@/hooks/usePlayback";
import { Play, Shuffle } from "lucide-react";
import * as tauri from "@/lib/tauri";
import type { Playlist, Track } from "@/types/track";

export function PlaylistPage() {
  const { id } = useParams<{ id: string }>();
  const [playlist, setPlaylist] = useState<Playlist | null>(null);
  const [tracks, setTracks] = useState<Track[]>([]);
  const [loading, setLoading] = useState(true);
  const { playTracks } = usePlayback();

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    Promise.all([tauri.getPlaylist(id), tauri.getPlaylistTracks(id)])
      .then(([pl, tr]) => {
        setPlaylist(pl);
        setTracks(tr);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [id]);

  if (loading) {
    return (
      <div className="flex flex-col gap-4 p-6">
        <Skeleton className="h-8 w-64" />
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-12 w-full" />
        ))}
      </div>
    );
  }

  if (!playlist) return null;

  return (
    <div className="flex flex-col gap-6 p-6">
      <div className="flex gap-6">
        {playlist.artworkUrl ? (
          <ProxiedImage
            src={playlist.artworkUrl}
            alt={playlist.name}
            className="size-48 rounded-sm object-cover shadow-sm"
            fallbackClassName="size-48 rounded-sm bg-muted"
          />
        ) : (
          <div className="size-48 rounded-sm bg-muted" />
        )}
        <div className="flex flex-col justify-end gap-1">
          <span className="text-xs/4 font-medium uppercase text-muted-foreground">
            Playlist
          </span>
          <h1 className="text-3xl/9 font-bold">{playlist.name}</h1>
          {playlist.description && (
            <p className="text-sm/5 text-muted-foreground">
              {playlist.description}
            </p>
          )}
          {playlist.numberOfItems != null && (
            <p className="text-sm/5 text-muted-foreground">
              {playlist.numberOfItems} tracks
            </p>
          )}
        </div>
      </div>
      <div className="flex gap-2">
        <Button
          size="sm"
          onClick={() => playTracks(tracks, 0)}
          disabled={tracks.length === 0}
        >
          <Play className="mr-1 size-4" />
          Play
        </Button>
        <Button
          size="sm"
          variant="outline"
          onClick={async () => {
            await playTracks(tracks, 0);
            await tauri.shuffleQueue();
          }}
          disabled={tracks.length === 0}
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
    </div>
  );
}
