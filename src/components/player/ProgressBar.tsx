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
      seek(newPos);
      setDragValue(null);
      isDragging.current = false;
      setDragging(false);
    },
    [duration, seek, setDragging],
  );

  const currentPos = dragValue ?? displayPosition;

  return (
    <div className="flex items-center gap-2">
      <span className="w-10 text-right text-xs/4 tabular-nums text-muted-foreground">
        {formatTime(currentPos)}
      </span>
      <Slider
        value={[fraction * 100]}
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
