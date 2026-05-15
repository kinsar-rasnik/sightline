/**
 * TimelineStackMarker — renders a multi-perspective moment on the lane
 * area as a single stack-silhouette card (ADR-0038 D5 + CEO-A2).
 *
 * A group forms when intervals overlap ≥ 50 % of the shorter interval
 * (see `groupMultiPerspective` in `lane-packing.ts`); the stack anchors
 * at the longest-running interval's position (D5). The silhouette
 * layers + avatar-dots row are the frozen Mission-2 `VodCard` `stack`
 * composition (DV4) — this component supplies the `stack` prop and
 * positions the result.
 *
 * A modifier click bulk-adds every perspective in the group to the
 * multi-selection (D7 Shift+Click-on-stack).
 */

import { memo, type CSSProperties, type MouseEvent } from "react";

import {
  VodCard,
  type VodCardStackPerspective,
} from "@/features/vods/VodCard";
import { useIsVodSelected } from "@/stores/timeline-multiselect-store";

import { variantForWidth } from "./TimelineCard";

function isModifiedClick(event: MouseEvent): boolean {
  return event.shiftKey || event.metaKey || event.ctrlKey;
}

interface TimelineStackMarkerProps {
  /** VOD of the longest-running interval — the stack anchors here. */
  leadVodId: string;
  /** Every member VOD id in the moment, longest-running first. */
  memberVodIds: string[];
  /** Streamer identities for the avatar-dots row (DV4). */
  perspectives: VodCardStackPerspective[];
  /** Card pixel width — lead interval duration × pxPerSecond. */
  widthPx: number;
  /** Horizontal offset from the lane-area left edge, in px (D6). */
  offsetPx: number;
  /** Vertical offset within the lane stack, in px. */
  topPx: number;
  /** Plain click — open the lead VOD (D5 default action). */
  onActivate: (vodId: string) => void;
  /** Modifier click — add every perspective to the selection (D7). */
  onToggleSelectGroup: (memberVodIds: string[]) => void;
}

function TimelineStackMarkerImpl({
  leadVodId,
  memberVodIds,
  perspectives,
  widthPx,
  offsetPx,
  topPx,
  onActivate,
  onToggleSelectGroup,
}: TimelineStackMarkerProps) {
  const selected = useIsVodSelected(leadVodId);

  const style: CSSProperties = {
    position: "absolute",
    top: topPx,
    left: 0,
    width: Math.max(widthPx, 1),
    transform: `translateX(${offsetPx}px)`,
  };

  const handleClickCapture = (event: MouseEvent): void => {
    if (!isModifiedClick(event)) return;
    event.preventDefault();
    event.stopPropagation();
    onToggleSelectGroup(memberVodIds);
  };

  return (
    <div
      style={style}
      data-timeline-stack={leadVodId}
      onClickCapture={handleClickCapture}
    >
      <VodCard
        vodId={leadVodId}
        variant={variantForWidth(widthPx)}
        selected={selected}
        stack={perspectives}
        onSelect={() => onActivate(leadVodId)}
      />
    </div>
  );
}

export const TimelineStackMarker = memo(TimelineStackMarkerImpl);
