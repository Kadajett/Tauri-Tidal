import { Play } from "lucide-react";
import { useNavigate } from "react-router";
import type { Track } from "@/types/track";
import { formatTime, cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";

interface TrackRowProps {
  track: Track;
  index: number;
  onPlay: (track: Track) => void;
  onContextMenu?: (e: React.MouseEvent, track: Track) => void;
}

export function TrackRow({ track, index, onPlay, onContextMenu }: TrackRowProps) {
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const isActive = currentTrack?.id === track.id;
  const navigate = useNavigate();

  return (
    <div
      className={cn(
        "group grid grid-cols-[2rem_1fr_1fr_4rem] items-center gap-4 rounded-xs px-3 py-2 hover:bg-accent/50",
        isActive && "bg-accent/30",
      )}
      onDoubleClick={() => onPlay(track)}
      onContextMenu={(e) => onContextMenu?.(e, track)}
    >
      <div className="flex items-center justify-center">
        <span className="text-sm tabular-nums text-muted-foreground group-hover:hidden">
          {isActive ? "~" : index + 1}
        </span>
        <button
          className="hidden size-4 items-center justify-center group-hover:flex"
          onClick={() => onPlay(track)}
        >
          <Play className="size-3" />
        </button>
      </div>
      <div className="min-w-0">
        <button
          className={cn(
            "truncate text-sm/5 text-left hover:underline hover:text-foreground",
            isActive && "text-primary font-medium",
          )}
          onClick={(e) => {
            e.stopPropagation();
            onPlay(track);
          }}
        >
          {track.title}
        </button>
        <p className="truncate text-xs/4 text-muted-foreground">
          {track.artistId ? (
            <button
              className="hover:underline hover:text-foreground"
              onClick={(e) => {
                e.stopPropagation();
                navigate(`/artist/${track.artistId}`);
              }}
            >
              {track.artistName}
            </button>
          ) : (
            track.artistName
          )}
        </p>
      </div>
      <div className="min-w-0">
        {track.albumId ? (
          <button
            className="truncate text-sm/5 text-muted-foreground text-left hover:underline hover:text-foreground"
            onClick={(e) => {
              e.stopPropagation();
              navigate(`/album/${track.albumId}`);
            }}
          >
            {track.albumName}
          </button>
        ) : (
          <p className="truncate text-sm/5 text-muted-foreground">
            {track.albumName}
          </p>
        )}
      </div>
      <p className="text-right text-sm/5 tabular-nums text-muted-foreground">
        {formatTime(track.duration)}
      </p>
    </div>
  );
}
