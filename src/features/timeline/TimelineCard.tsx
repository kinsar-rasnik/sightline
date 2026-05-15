/**
 * TimelineCard — a single-VOD card positioned on the Single-Time-Axis
 * timeline (ADR-0038). A thin wrapper around the frozen Mission-2
 * `VodCard`: it picks the width variant (TV-A2), positions the card on
 * the time axis (D6 `translateX`), and routes D7 click gestures.
 *
 * Plain click opens the VOD; a modifier click (Shift / Cmd / Ctrl)
 * toggles multi-selection. The modifier branch is intercepted in the
 * capture phase so the inner VodCard button's plain-click handler never
 * fires for a modified click.
 */

import { memo, type CSSProperties, type MouseEvent } from "react";

import { VodCard, type VodCardVariant } from "@/features/vods/VodCard";
import { useIsVodSelected } from "@/stores/timeline-multiselect-store";

/** TV-A2: Full ↔ Compact threshold. */
export const CARD_WIDTH_FULL_PX = 160;
/** TV-A2: Compact ↔ Sliver threshold (= image natural width). */
export const CARD_WIDTH_COMPACT_PX = 96;

/** TV-A2: map a card's pixel width onto a VodCard width variant. */
export function variantForWidth(widthPx: number): VodCardVariant {
  if (widthPx >= CARD_WIDTH_FULL_PX) return "full";
  if (widthPx >= CARD_WIDTH_COMPACT_PX) return "compact";
  return "sliver";
}

/** D7: a Shift / Cmd / Ctrl click is a selection gesture, not an open. */
export function isModifiedClick(event: MouseEvent): boolean {
  return event.shiftKey || event.metaKey || event.ctrlKey;
}

interface TimelineCardProps {
  vodId: string;
  /** Card pixel width — interval duration × pxPerSecond. */
  widthPx: number;
  /** Horizontal offset from the lane-area left edge, in px (D6). */
  offsetPx: number;
  /** Vertical offset within the lane stack, in px. */
  topPx: number;
  /** Plain click — open the VOD (D7 default action). */
  onActivate: (vodId: string) => void;
  /** Shift / Cmd / Ctrl click — toggle multi-selection (D7). */
  onToggleSelect: (vodId: string) => void;
}

function TimelineCardImpl({
  vodId,
  widthPx,
  offsetPx,
  topPx,
  onActivate,
  onToggleSelect,
}: TimelineCardProps) {
  const selected = useIsVodSelected(vodId);
  const variant = variantForWidth(widthPx);

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
    onToggleSelect(vodId);
  };

  return (
    <div
      style={style}
      data-timeline-card={vodId}
      onClickCapture={handleClickCapture}
    >
      <VodCard
        vodId={vodId}
        variant={variant}
        selected={selected}
        onSelect={() => onActivate(vodId)}
      />
    </div>
  );
}

export const TimelineCard = memo(TimelineCardImpl);
