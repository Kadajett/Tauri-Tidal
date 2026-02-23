import { useEffect, useRef } from "react";
import { useNavigate } from "react-router";
import type { Track } from "@/types/track";
import { useLibraryStore } from "@/stores/libraryStore";
import { useLibrary } from "@/hooks/useLibrary";
import * as tauri from "@/lib/tauri";

interface TrackContextMenuProps {
  track: Track;
  x: number;
  y: number;
  onClose: () => void;
}

export function TrackContextMenu({
  track,
  x,
  y,
  onClose,
}: TrackContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();
  const playlists = useLibraryStore((s) => s.playlists);
  const isFavorite = useLibraryStore((s) => s.isFavorite);
  const { toggleFavorite, addToPlaylist } = useLibrary();
  const favorited = isFavorite(track.id);

  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [onClose]);

  const menuItems = [
    {
      label: "Add to Queue",
      action: () => tauri.addToQueue(track.id),
    },
    ...(track.albumId
      ? [
          {
            label: "Go to Album",
            action: () => navigate(`/album/${track.albumId}`),
          },
        ]
      : []),
    ...(track.artistId
      ? [
          {
            label: "Go to Artist",
            action: () => navigate(`/artist/${track.artistId}`),
          },
        ]
      : []),
    {
      label: favorited ? "Remove from Favorites" : "Add to Favorites",
      action: () => toggleFavorite(track.id, favorited),
    },
    {
      label: "Similar Tracks",
      action: () => navigate(`/similar?trackId=${track.id}`),
    },
  ];

  return (
    <div
      ref={ref}
      className="fixed z-50 min-w-48 rounded-sm border border-border bg-popover p-1 shadow-sm"
      style={{ left: x, top: y }}
    >
      {menuItems.map((item) => (
        <button
          key={item.label}
          className="flex w-full items-center rounded-xs px-3 py-1.5 text-sm/5 hover:bg-accent"
          onClick={() => {
            item.action();
            onClose();
          }}
        >
          {item.label}
        </button>
      ))}

      {playlists.length > 0 && (
        <>
          <div className="my-1 h-px bg-border" />
          <div className="px-3 py-1 text-xs/4 text-muted-foreground">
            Add to Playlist
          </div>
          {playlists.map((pl) => (
            <button
              key={pl.id}
              className="flex w-full items-center rounded-xs px-3 py-1.5 text-sm/5 hover:bg-accent"
              onClick={() => {
                addToPlaylist(pl.id, track.id);
                onClose();
              }}
            >
              {pl.name}
            </button>
          ))}
        </>
      )}
    </div>
  );
}
