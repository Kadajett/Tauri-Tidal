import { TrackInfo } from "./TrackInfo";
import { PlayerControls } from "./PlayerControls";
import { ProgressBar } from "./ProgressBar";
import { VolumeControl } from "./VolumeControl";

export function FooterPlayer() {
  return (
    <div className="border-t border-border bg-card px-4 py-2">
      <div className="grid grid-cols-[1fr_2fr_1fr] items-center gap-4">
        <TrackInfo />
        <div className="flex flex-col gap-1">
          <PlayerControls />
          <ProgressBar />
        </div>
        <VolumeControl />
      </div>
    </div>
  );
}
