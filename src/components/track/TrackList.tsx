import type { Track } from "@/types/track";
import { TrackRow } from "./TrackRow";
import { TrackContextMenu } from "./TrackContextMenu";
import { useState, useCallback } from "react";

interface TrackListProps {
  tracks: Track[];
  onPlay: (track: Track) => void;
}

export function TrackList({ tracks, onPlay }: TrackListProps) {
  const [contextMenu, setContextMenu] = useState<{
    track: Track;
    x: number;
    y: number;
  } | null>(null);

  const handleContextMenu = useCallback((e: React.MouseEvent, track: Track) => {
    e.preventDefault();
    setContextMenu({ track, x: e.clientX, y: e.clientY });
  }, []);

  return (
    <>
      <div className="grid grid-cols-[2rem_1fr_1fr_4rem] items-center gap-4 border-b border-border px-3 py-2 text-xs/4 font-medium uppercase text-muted-foreground">
        <span className="text-center">#</span>
        <span>Title</span>
        <span>Album</span>
        <span className="text-right">Time</span>
      </div>
      <div className="flex flex-col">
        {tracks.map((track, i) => (
          <TrackRow
            key={track.id}
            track={track}
            index={i}
            onPlay={onPlay}
            onContextMenu={handleContextMenu}
          />
        ))}
      </div>
      {contextMenu && (
        <TrackContextMenu
          track={contextMenu.track}
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  );
}
