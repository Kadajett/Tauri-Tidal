import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router";
import { Button } from "@/components/ui/button";
import { TrackList } from "@/components/track/TrackList";
import { Skeleton } from "@/components/ui/skeleton";
import { usePlayback } from "@/hooks/usePlayback";
import { Play, Shuffle } from "lucide-react";
import * as tauri from "@/lib/tauri";
import type { Album, Track } from "@/types/track";

export function AlbumPage() {
  const { id } = useParams<{ id: string }>();
  const [album, setAlbum] = useState<Album | null>(null);
  const [tracks, setTracks] = useState<Track[]>([]);
  const [loading, setLoading] = useState(true);
  const { playTracks } = usePlayback();
  const navigate = useNavigate();

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    Promise.all([tauri.getAlbum(id), tauri.getAlbumTracks(id)])
      .then(([albumData, trackData]) => {
        setAlbum(albumData);
        setTracks(trackData);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [id]);

  if (loading) {
    return (
      <div className="flex flex-col gap-4 p-6">
        <div className="flex gap-6">
          <Skeleton className="size-48" />
          <div className="flex flex-col gap-2">
            <Skeleton className="h-8 w-64" />
            <Skeleton className="h-5 w-40" />
          </div>
        </div>
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-12 w-full" />
        ))}
      </div>
    );
  }

  if (!album) return null;

  return (
    <div className="flex flex-col gap-6 p-6">
      <div className="flex gap-6">
        {album.artworkUrl ? (
          <img
            src={album.artworkUrl}
            alt={album.title}
            className="size-48 rounded-sm object-cover shadow-sm"
            referrerPolicy="no-referrer"
          />
        ) : (
          <div className="size-48 rounded-sm bg-muted" />
        )}
        <div className="flex flex-col justify-end gap-1">
          <span className="text-xs/4 font-medium uppercase text-muted-foreground">
            Album
          </span>
          <h1 className="text-3xl/9 font-bold">{album.title}</h1>
          <p className="text-sm/5 text-muted-foreground">
            {album.artistId ? (
              <button
                className="hover:underline hover:text-foreground"
                onClick={() => navigate(`/artist/${album.artistId}`)}
              >
                {album.artistName}
              </button>
            ) : (
              album.artistName
            )}
            {album.releaseDate && ` \u00B7 ${album.releaseDate.slice(0, 4)}`}
            {album.numberOfTracks != null &&
              ` \u00B7 ${album.numberOfTracks} tracks`}
          </p>
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
