import { memo } from "react";
import { MobileTopBar } from "./MobileTopBar";
import { QueueBody } from "./queue/QueueBody";
import type { MobileScreenProps } from "./types";

/** Queue tab (Figma E2 Queue — Up Next). */
export const MobileQueue = memo(function MobileQueue({ c }: MobileScreenProps) {
  return (
    <div className="m-screen__container">
      <MobileTopBar num="04" name="QUEUE" />
      <div className="m-screen__content m-screen__content--list m-queue">
        <QueueBody c={c} kicker="04 // QUEUE" />
      </div>
    </div>
  );
});
