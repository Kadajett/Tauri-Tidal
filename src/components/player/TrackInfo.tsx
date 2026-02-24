import { Heart } from "lucide-react";
import { useNavigate } from "react-router";
import { Button } from "@/components/ui/button";
import { ProxiedImage } from "@/components/ui/proxied-image";
import { usePlayerStore } from "@/stores/playerStore";
import { useLibraryStore } from "@/stores/libraryStore";
import { useLibrary } from "@/hooks/useLibrary";

export function TrackInfo() {
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const quality = usePlayerStore((s) => s.quality);
  const setExpanded = usePlayerStore((s) => s.setExpanded);
  const isFavorite = useLibraryStore((s) => s.isFavorite);
  const { toggleFavorite } = useLibrary();
  const navigate = useNavigate();

  if (!currentTrack) {
    return <div className="flex items-center gap-3" />;
  }

  const favorited = isFavorite(currentTrack.id);

  const artwork = currentTrack.artworkUrl ? (
    <ProxiedImage
      src={currentTrack.artworkUrl}
      alt={currentTrack.albumName}
      className="size-12 rounded-xs object-cover"
      fallbackClassName="size-12 rounded-xs bg-muted"
    />
  ) : (
    <div className="size-12 rounded-xs bg-muted" />
  );

  return (
    <div className="flex items-center gap-3 overflow-hidden">
      <button
        className="shrink-0"
        onClick={() => setExpanded(true)}
      >
        {artwork}
      </button>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm/5 font-medium">{currentTrack.title}</p>
        <p className="flex items-center gap-1.5 truncate text-xs/4 text-muted-foreground">
          {currentTrack.artistId ? (
            <button
              className="truncate hover:underline hover:text-foreground"
              onClick={() => navigate(`/artist/${currentTrack.artistId}`)}
            >
              {currentTrack.artistName}
            </button>
          ) : (
            <span className="truncate">{currentTrack.artistName}</span>
          )}
          {quality && (
            <span className="shrink-0 rounded-xs border border-border px-1 py-px text-[10px]/3 font-medium uppercase text-muted-foreground">
              {quality}
            </span>
          )}
        </p>
      </div>
      <Button
        variant="ghost"
        size="icon"
        className="size-8 shrink-0"
        onClick={() => toggleFavorite(currentTrack.id, favorited)}
      >
        <Heart
          className={`size-4 ${favorited ? "fill-current text-red-500" : ""}`}
        />
      </Button>
    </div>
  );
}
