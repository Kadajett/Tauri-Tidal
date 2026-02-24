import { useNavigate } from "react-router";
import type { Artist } from "@/types/track";

interface ArtistCardProps {
  artist: Artist;
}

export function ArtistCard({ artist }: ArtistCardProps) {
  const navigate = useNavigate();

  return (
    <button
      className="flex flex-col items-center gap-2 rounded-sm p-3 text-center hover:bg-accent/50"
      onClick={() => navigate(`/artist/${artist.id}`)}
    >
      {artist.pictureUrl ? (
        <img
          src={artist.pictureUrl}
          alt={artist.name}
          className="aspect-square w-full rounded-full object-cover"
          referrerPolicy="no-referrer"
        />
      ) : (
        <div className="aspect-square w-full rounded-full bg-muted" />
      )}
      <p className="truncate text-sm/5 font-medium">{artist.name}</p>
    </button>
  );
}
