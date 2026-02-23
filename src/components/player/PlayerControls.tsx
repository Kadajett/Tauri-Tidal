import { Play, Pause, SkipBack, SkipForward } from "lucide-react";
import { Button } from "@/components/ui/button";
import { usePlayback } from "@/hooks/usePlayback";

export function PlayerControls() {
  const { togglePlayPause, nextTrack, previousTrack, isPlaying } =
    usePlayback();

  return (
    <div className="flex items-center justify-center gap-2">
      <Button
        variant="ghost"
        size="icon"
        className="size-8"
        onClick={previousTrack}
      >
        <SkipBack className="size-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="size-9 rounded-full bg-primary text-primary-foreground hover:bg-primary/90"
        onClick={togglePlayPause}
      >
        {isPlaying ? (
          <Pause className="size-4" />
        ) : (
          <Play className="size-4 ml-0.5" />
        )}
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="size-8"
        onClick={nextTrack}
      >
        <SkipForward className="size-4" />
      </Button>
    </div>
  );
}
