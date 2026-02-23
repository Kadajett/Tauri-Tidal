import { useNavigate } from "react-router";
import type { Playlist } from "@/types/track";

interface PlaylistCardProps {
  playlist: Playlist;
}

export function PlaylistCard({ playlist }: PlaylistCardProps) {
  const navigate = useNavigate();

  return (
    <button
      className="flex flex-col gap-2 rounded-sm p-3 text-left hover:bg-accent/50"
      onClick={() => navigate(`/playlist/${playlist.id}`)}
    >
      {playlist.artworkUrl ? (
        <img
          src={playlist.artworkUrl}
          alt={playlist.name}
          className="aspect-square w-full rounded-xs object-cover"
        />
      ) : (
        <div className="aspect-square w-full rounded-xs bg-muted" />
      )}
      <p className="truncate text-sm/5 font-medium">{playlist.name}</p>
      {playlist.numberOfItems != null && (
        <p className="text-xs/4 text-muted-foreground">
          {playlist.numberOfItems} tracks
        </p>
      )}
    </button>
  );
}
