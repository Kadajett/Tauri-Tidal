import { useNavigate } from "react-router";
import type { Album } from "@/types/track";

interface AlbumCardProps {
  album: Album;
}

export function AlbumCard({ album }: AlbumCardProps) {
  const navigate = useNavigate();

  return (
    <button
      className="flex flex-col gap-2 rounded-sm p-3 text-left hover:bg-accent/50"
      onClick={() => navigate(`/album/${album.id}`)}
    >
      {album.artworkUrl ? (
        <img
          src={album.artworkUrl}
          alt={album.title}
          className="aspect-square w-full rounded-xs object-cover"
        />
      ) : (
        <div className="aspect-square w-full rounded-xs bg-muted" />
      )}
      <p className="truncate text-sm/5 font-medium">{album.title}</p>
      <p className="truncate text-xs/4 text-muted-foreground">
        {album.artistName}
      </p>
    </button>
  );
}
