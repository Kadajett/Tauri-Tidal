import { Volume2, VolumeX, Volume1 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { usePlayerStore } from "@/stores/playerStore";
import { usePlayback } from "@/hooks/usePlayback";

export function VolumeControl() {
  const volume = usePlayerStore((s) => s.volume);
  const muted = usePlayerStore((s) => s.muted);
  const { setVolume, toggleMute } = usePlayback();

  const VolumeIcon = muted || volume === 0 ? VolumeX : volume < 0.5 ? Volume1 : Volume2;

  return (
    <div className="flex items-center justify-end gap-2">
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
        className="w-24"
      />
    </div>
  );
}
