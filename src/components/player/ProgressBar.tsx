import { useCallback, useRef, useState } from "react";
import { Slider } from "@/components/ui/slider";
import { useAnimatedProgress } from "@/hooks/useAnimatedProgress";
import { usePlayback } from "@/hooks/usePlayback";
import { formatTime } from "@/lib/utils";

export function ProgressBar() {
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
      // Set display position to the seek target BEFORE clearing drag,
      // so the animation re-syncs from here instead of snapping back
      setDisplayPosition(newPos);
      seek(newPos);
      setDragValue(null);
      isDragging.current = false;
      setDragging(false);
    },
    [duration, seek, setDragging, setDisplayPosition],
  );

  const currentPos = dragValue ?? displayPosition;
  const sliderValue = dragValue != null && duration > 0
    ? (dragValue / duration) * 100
    : fraction * 100;

  return (
    <div className="flex items-center gap-2">
      <span className="w-10 text-right text-xs/4 tabular-nums text-muted-foreground">
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
      <span className="w-10 text-xs/4 tabular-nums text-muted-foreground">
        {formatTime(duration)}
      </span>
    </div>
  );
}
