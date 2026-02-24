import { useCallback, useEffect, useRef, useState } from "react";
import {
  ChevronDown,
  Heart,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
  Volume1,
} from "lucide-react";
import { useNavigate } from "react-router";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { ProxiedImage } from "@/components/ui/proxied-image";
import { TrackList } from "@/components/track/TrackList";
import { usePlayerStore } from "@/stores/playerStore";
import { useQueueStore } from "@/stores/queueStore";
import { useLibraryStore } from "@/stores/libraryStore";
import { useLibrary } from "@/hooks/useLibrary";
import { usePlayback } from "@/hooks/usePlayback";
import { useAnimatedProgress } from "@/hooks/useAnimatedProgress";
import { formatTime } from "@/lib/utils";
import * as tauri from "@/lib/tauri";

function FullPageProgress() {
  const { displayPosition, fraction, duration, setDragging, setDisplayPosition } =
    useAnimatedProgress();
  const { seek } = usePlayback();
  const [dragValue, setDragValue] = useState<number | null>(null);
  const isDragging = useRef(false);

  const handleValueChange = useCallback(
    (value: number[]) => {
      const newPos = (value[0] / 100) * duration;
      setDragValue(newPos);
      setDisplayPosition(newPos);
    },
    [duration, setDisplayPosition],
  );

  const handlePointerDown = useCallback(() => {
    isDragging.current = true;
    setDragging(true);
  }, [setDragging]);

  const handleValueCommit = useCallback(
    (value: number[]) => {
      const newPos = (value[0] / 100) * duration;
      setDisplayPosition(newPos);
      seek(newPos);
      setDragValue(null);
      isDragging.current = false;
      setDragging(false);
    },
    [duration, seek, setDragging, setDisplayPosition],
  );

  const currentPos = dragValue ?? displayPosition;
  const sliderValue =
    dragValue != null && duration > 0
      ? (dragValue / duration) * 100
      : fraction * 100;

  return (
    <div className="flex w-full items-center gap-3">
      <span className="w-12 text-right text-sm/5 tabular-nums text-muted-foreground">
        {formatTime(currentPos)}
      </span>
      <Slider
        value={[sliderValue]}
        max={100}
        step={0.1}
        onValueChange={handleValueChange}
        onValueCommit={handleValueCommit}
        onPointerDown={handlePointerDown}
        className="flex-1"
      />
      <span className="w-12 text-sm/5 tabular-nums text-muted-foreground">
        {formatTime(duration)}
      </span>
    </div>
  );
}

export function FullPagePlayer() {
  const expanded = usePlayerStore((s) => s.expanded);
  const setExpanded = usePlayerStore((s) => s.setExpanded);
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const quality = usePlayerStore((s) => s.quality);
  const volume = usePlayerStore((s) => s.volume);
  const muted = usePlayerStore((s) => s.muted);
  const queueTracks = useQueueStore((s) => s.tracks);
  const isFavorite = useLibraryStore((s) => s.isFavorite);
  const { toggleFavorite } = useLibrary();
  const { togglePlayPause, nextTrack, previousTrack, isPlaying, setVolume, toggleMute } =
    usePlayback();
  const navigate = useNavigate();

  // Close on Escape
  useEffect(() => {
    if (!expanded) return;
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setExpanded(false);
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [expanded, setExpanded]);

  const favorited = currentTrack ? isFavorite(currentTrack.id) : false;

  // Get 1280x1280 artwork for the full-page view
  const hiResArtwork = currentTrack?.artworkUrl?.replace(
    /\/\d+x\d+\.jpg$/,
    "/1280x1280.jpg",
  );

  const VolumeIcon =
    muted || volume === 0 ? VolumeX : volume < 0.5 ? Volume1 : Volume2;

  const handleQueuePlay = useCallback(
    (track: { id: string }) => {
      const idx = queueTracks.findIndex((t) => t.id === track.id);
      if (idx >= 0) tauri.playQueueTrack(idx);
    },
    [queueTracks],
  );

  return (
    <div
      className={`fixed inset-0 z-50 flex flex-col bg-background transition-transform duration-300 ease-out ${
        expanded ? "translate-y-0" : "translate-y-full"
      }`}
    >
      {/* Header bar */}
      <div className="flex items-center justify-between px-6 py-3">
        <Button
          variant="ghost"
          size="icon"
          className="size-10"
          onClick={() => setExpanded(false)}
        >
          <ChevronDown className="size-6" />
        </Button>
        <p className="text-xs/4 uppercase tracking-wider text-muted-foreground">
          Now Playing
        </p>
        <div className="size-10" />
      </div>

      {/* Two-column layout: player left, queue right */}
      <div className="flex min-h-0 flex-1">
        {/* Left column: player */}
        <div className="flex flex-1 flex-col items-center justify-center gap-6 px-8 pb-12">
          {/* Album art (shrunk slightly) */}
          <div className="w-full max-w-sm">
            {hiResArtwork ? (
              <ProxiedImage
                src={hiResArtwork}
                alt={currentTrack?.albumName ?? ""}
                className="aspect-square w-full rounded-sm object-cover shadow-lg"
                fallbackClassName="aspect-square w-full rounded-sm bg-muted"
              />
            ) : (
              <div className="aspect-square w-full rounded-sm bg-muted" />
            )}
          </div>

          {/* Track info */}
          <div className="flex w-full max-w-sm items-start justify-between gap-4">
            <div className="min-w-0 flex-1">
              <h2 className="truncate text-xl/7 font-semibold">
                {currentTrack?.title}
              </h2>
              <p className="flex items-center gap-2 text-base/6 text-muted-foreground">
                {currentTrack?.artistId ? (
                  <button
                    className="truncate hover:text-foreground hover:underline"
                    onClick={() => {
                      setExpanded(false);
                      navigate(`/artist/${currentTrack.artistId}`);
                    }}
                  >
                    {currentTrack.artistName}
                  </button>
                ) : (
                  <span className="truncate">{currentTrack?.artistName}</span>
                )}
                {currentTrack?.albumId && (
                  <>
                    <span className="text-muted-foreground/50">&middot;</span>
                    <button
                      className="truncate hover:text-foreground hover:underline"
                      onClick={() => {
                        setExpanded(false);
                        navigate(`/album/${currentTrack.albumId}`);
                      }}
                    >
                      {currentTrack.albumName}
                    </button>
                  </>
                )}
              </p>
              {quality && (
                <span className="mt-1 inline-block rounded-xs border border-border px-1.5 py-0.5 text-xs/3 font-medium uppercase text-muted-foreground">
                  {quality}
                </span>
              )}
            </div>
            {currentTrack && (
              <Button
                variant="ghost"
                size="icon"
                className="size-10 shrink-0"
                onClick={() => toggleFavorite(currentTrack.id, favorited)}
              >
                <Heart
                  className={`size-5 ${favorited ? "fill-current text-red-500" : ""}`}
                />
              </Button>
            )}
          </div>

          {/* Progress bar */}
          <div className="w-full max-w-sm">
            <FullPageProgress />
          </div>

          {/* Playback controls */}
          <div className="flex items-center gap-6">
            <Button
              variant="ghost"
              size="icon"
              className="size-12"
              onClick={previousTrack}
            >
              <SkipBack className="size-6" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="size-14 rounded-full bg-primary text-primary-foreground hover:bg-primary/90"
              onClick={togglePlayPause}
            >
              {isPlaying ? (
                <Pause className="size-7" />
              ) : (
                <Play className="size-7 ml-0.5" />
              )}
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="size-12"
              onClick={nextTrack}
            >
              <SkipForward className="size-6" />
            </Button>
          </div>

          {/* Volume */}
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              className="size-8"
              onClick={toggleMute}
            >
              <VolumeIcon className="size-4" />
            </Button>
            <Slider
              value={[volume * 100]}
              max={100}
              step={1}
              onValueChange={(v) => setVolume(v[0] / 100)}
              className="w-32"
            />
          </div>
        </div>

        {/* Right column: queue */}
        <div className="flex w-96 flex-col border-l border-border">
          <div className="px-4 py-3">
            <h3 className="text-sm/5 font-semibold uppercase tracking-wider text-muted-foreground">
              Queue
            </h3>
          </div>
          <div className="min-h-0 flex-1 overflow-auto px-1">
            {queueTracks.length > 0 ? (
              <TrackList
                tracks={queueTracks}
                showArtwork
                onPlay={handleQueuePlay}
              />
            ) : (
              <p className="px-4 py-8 text-center text-sm/5 text-muted-foreground">
                Queue is empty
              </p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
