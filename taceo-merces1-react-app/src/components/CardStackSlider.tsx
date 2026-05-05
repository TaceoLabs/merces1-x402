import * as React from "react";
import styles from "./card-stack-slider.module.css";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let scrollTimelinePolyfillPromise: Promise<any> | null = null;

function supportsNativeScrollDrivenAnimations() {
  if (typeof window === "undefined") return true;
  return Boolean(
    "ScrollTimeline" in window &&
      "ViewTimeline" in window &&
      typeof CSS !== "undefined" &&
      CSS.supports?.("animation-timeline: view()") &&
      CSS.supports?.("view-timeline-name: --stack-item"),
  );
}

async function ensureScrollTimelinePolyfill() {
  if (supportsNativeScrollDrivenAnimations()) return;
  if (!scrollTimelinePolyfillPromise) {
    scrollTimelinePolyfillPromise = import(
      // @ts-expect-error no types for this package
      "scroll-timeline-polyfill/dist/scroll-timeline.js"
    );
  }
  await scrollTimelinePolyfillPromise;
}

type StackSliderStyle = React.CSSProperties & {
  "--stack-inactive-scale"?: number | string;
  "--stack-hover-scale"?: number | string;
  "--stack-side-peek"?: string;
  "--stack-edge-translate"?: string;
  "--stack-edge-space"?: string;
  "--stack-z-active"?: number | string;
  "--stack-z-inactive"?: number | string;
  "--stack-item-width"?: string;
  "--stack-overlay-color"?: string;
  "--stack-overlay-opacity"?: number | string;
};

export type CardStackSliderSource = "scroll" | "click" | "programmatic";

export type CardStackSliderActiveCardMeta = {
  source: CardStackSliderSource;
};

export type CardStackSliderHandle = {
  scrollToIndex: (index: number, behavior?: ScrollBehavior) => void;
};

type CardStackSliderProps = Omit<
  React.ComponentPropsWithoutRef<"section">,
  "children" | "style"
> & {
  children: React.ReactNode;
  initialIndex?: number;
  viewportClassName?: string;
  itemClassName?: string;
  style?: StackSliderStyle;
  inactiveScale?: number;
  hoverScale?: number;
  sidePeek?: string;
  edgeTranslate?: string;
  activeZ?: number;
  inactiveZ?: number;
  overlayColor?: string;
  overlayOpacity?: number;
  activeCardChangeDebounceMs?: number;
  onActiveCardChange?: (
    card: HTMLLIElement,
    index: number,
    meta: CardStackSliderActiveCardMeta,
  ) => void;
};

export const CardStackSlider = React.forwardRef<
  CardStackSliderHandle,
  CardStackSliderProps
>(function CardStackSlider(
  {
    children,
    initialIndex = 0,
    className,
    viewportClassName,
    itemClassName,
    style,
    inactiveScale,
    hoverScale,
    sidePeek,
    edgeTranslate,
    activeZ,
    inactiveZ,
    overlayColor,
    overlayOpacity,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    activeCardChangeDebounceMs: _activeCardChangeDebounceMs,
    onActiveCardChange,
    ...props
  },
  ref,
) {
  const items = React.Children.toArray(children);
  const viewportRef = React.useRef<HTMLDivElement>(null);
  const trackRef = React.useRef<HTMLOListElement>(null);
  const itemRefs = React.useRef<Array<HTMLLIElement | null>>([]);
  const itemInnerRefs = React.useRef<Array<HTMLDivElement | null>>([]);
  const overlayRefs = React.useRef<Array<HTMLDivElement | null>>([]);
  const [isReady, setIsReady] = React.useState(false);
  const [activeIndex, setActiveIndex] = React.useState<number | null>(null);
  const [isHoverCapable, setIsHoverCapable] = React.useState(false);
  const lastEmittedActiveIndexRef = React.useRef<number | null>(null);
  const pendingActiveSourceRef = React.useRef<CardStackSliderSource>("scroll");
  const hasInitializedRef = React.useRef(false);
  const ACTIVE_INDEX_HYSTERESIS_PX = 8;
  type SnapLikeEvent = Event & {
    snapTargetBlock?: EventTarget | null;
    snapTargetInline?: EventTarget | null;
  };

  const getClampedIndex = React.useCallback(
    (index: number) =>
      Math.min(Math.max(index, 0), Math.max(items.length - 1, 0)),
    [items.length],
  );

  const scrollItemIntoView = React.useCallback(
    (index: number, behavior: ScrollBehavior, source: CardStackSliderSource = "scroll") => {
      const viewport = viewportRef.current;
      const targetIndex = getClampedIndex(index);
      const targetItem = itemRefs.current[targetIndex];
      if (!viewport || !targetItem) return;
      pendingActiveSourceRef.current = source;
      targetItem.scrollIntoView({ behavior, block: "nearest", inline: "center" });
    },
    [getClampedIndex],
  );

  React.useImperativeHandle(
    ref,
    () => ({
      scrollToIndex(index, behavior = "smooth") {
        scrollItemIntoView(index, behavior, "programmatic");
      },
    }),
    [scrollItemIntoView],
  );

  React.useEffect(() => {
    void ensureScrollTimelinePolyfill();
  }, []);

  React.useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const mediaQuery = window.matchMedia("(hover: hover) and (any-pointer: fine)");
    const update = () => setIsHoverCapable(mediaQuery.matches);
    update();
    mediaQuery.addEventListener("change", update);
    return () => mediaQuery.removeEventListener("change", update);
  }, []);

  React.useLayoutEffect(() => {
    if (hasInitializedRef.current) return;
    const viewport = viewportRef.current;
    if (!viewport || items.length === 0) {
      setIsReady(true);
      hasInitializedRef.current = true;
      return;
    }
    setIsReady(false);
    const clampedIndex = getClampedIndex(initialIndex);
    let revealFrame = 0;
    const frame = requestAnimationFrame(() => {
      scrollItemIntoView(clampedIndex, "auto", "programmatic");
      revealFrame = requestAnimationFrame(() => {
        setIsReady(true);
        hasInitializedRef.current = true;
      });
    });
    return () => {
      cancelAnimationFrame(frame);
      if (revealFrame !== 0) cancelAnimationFrame(revealFrame);
    };
  }, [getClampedIndex, initialIndex, items.length, scrollItemIntoView]);

  React.useLayoutEffect(() => {
    const viewport = viewportRef.current;
    const track = trackRef.current;
    if (!viewport || !track || items.length === 0) return;

    const supportsScrollSnapChanging =
      typeof window !== "undefined" &&
      "SnapEvent" in window &&
      "onscrollsnapchanging" in document.createElement("div");

    const getSliderRoot = () =>
      viewport.closest(`.${styles.root}`) as HTMLElement | null;

    const updateEdgeSpace = () => {
      const sliderRoot = getSliderRoot();
      const firstItem = itemRefs.current[0];
      if (!sliderRoot || !track || !firstItem) return;
      const visibleItems = itemRefs.current.filter(
        (item): item is HTMLLIElement => item instanceof HTMLLIElement,
      );
      const firstRect = firstItem.getBoundingClientRect();
      const itemWidth = firstItem.getBoundingClientRect().width;
      const edgeSpace = Math.max(0, (viewport.clientWidth - itemWidth) / 2);
      const secondItem = visibleItems[1];
      const step =
        secondItem === undefined
          ? itemWidth
          : Math.abs(
              secondItem.getBoundingClientRect().left +
                secondItem.getBoundingClientRect().width / 2 -
                (firstRect.left + firstRect.width / 2),
            );
      const trackWidth = Math.max(
        viewport.clientWidth,
        edgeSpace * 2 + itemWidth + step * Math.max(visibleItems.length - 1, 0),
      );
      sliderRoot.style.setProperty("--stack-edge-space", `${edgeSpace}px`);
      track.style.width = `${trackWidth}px`;
    };

    let frame = 0;
    let resizeObserver: ResizeObserver | null = null;

    const getVisualCenters = () => {
      const viewportRect = viewport.getBoundingClientRect();
      const viewportCenter = viewportRect.left + viewportRect.width / 2;
      const centers: number[] = [];
      const distances: number[] = [];
      itemRefs.current.forEach((item, index) => {
        if (!item) return;
        const visualTarget = itemInnerRefs.current[index] ?? item;
        const rect = visualTarget.getBoundingClientRect();
        const center = rect.left + rect.width / 2;
        centers[index] = center;
        distances[index] = Math.abs(center - viewportCenter);
      });
      return { centers, distances, viewportCenter };
    };

    const commitActiveFromVisualCenter = (source: CardStackSliderSource = "scroll") => {
      const { distances } = getVisualCenters();
      let closestIndex = -1;
      let closestDistance = Number.POSITIVE_INFINITY;
      distances.forEach((distance, index) => {
        if (distance < closestDistance) {
          closestDistance = distance;
          closestIndex = index;
        }
      });
      if (closestIndex === -1) return;
      setActiveIndex((current) => {
        if (current === closestIndex) return current;
        if (typeof current === "number") {
          const currentDistance = distances[current];
          if (
            typeof currentDistance === "number" &&
            closestDistance + ACTIVE_INDEX_HYSTERESIS_PX >= currentDistance
          ) {
            return current;
          }
        }
        pendingActiveSourceRef.current = source;
        return closestIndex;
      });
    };

    const commitActiveFromSnapTarget = (event: Event) => {
      const snapEvent = event as SnapLikeEvent;
      const snapTarget =
        (snapEvent.snapTargetInline instanceof Element && snapEvent.snapTargetInline) ||
        (snapEvent.snapTargetBlock instanceof Element && snapEvent.snapTargetBlock) ||
        null;
      if (!snapTarget) return;
      const snapItem = snapTarget.closest("li");
      if (!snapItem) return;
      const nextIndex = itemRefs.current.findIndex((item) => item === snapItem);
      if (nextIndex === -1) return;
      setActiveIndex((current) => {
        if (current === nextIndex) return current;
        pendingActiveSourceRef.current = "scroll";
        return nextIndex;
      });
    };

    const updateOverlayOpacity = () => {
      frame = 0;
      const slider = getSliderRoot();
      const sliderStyles = slider ? getComputedStyle(slider) : null;
      const overlayOpacityValue = Number.parseFloat(
        sliderStyles?.getPropertyValue("--stack-overlay-opacity") ?? "",
      );
      const maxOverlayOpacity = Number.isNaN(overlayOpacityValue) ? 0.5 : overlayOpacityValue;
      const { centers, distances } = getVisualCenters();
      let stepDistance = 0;
      let previousCenter: number | null = null;
      centers.forEach((center) => {
        if (typeof center !== "number") return;
        if (previousCenter !== null && stepDistance === 0) {
          stepDistance = Math.abs(center - previousCenter);
        }
        previousCenter = center;
      });
      const normalizedStep =
        stepDistance > 0 ? stepDistance : Math.max(1, viewport.clientWidth / 2);
      overlayRefs.current.forEach((overlay, index) => {
        if (!overlay) return;
        const distance = distances[index];
        if (typeof distance !== "number") return;
        const ratio = Math.min(1, distance / normalizedStep);
        overlay.style.opacity = `${ratio * maxOverlayOpacity}`;
      });
    };

    const scheduleUpdate = () => {
      if (frame !== 0) return;
      frame = requestAnimationFrame(updateOverlayOpacity);
    };

    const handleScroll = () => {
      scheduleUpdate();
      if (!supportsScrollSnapChanging) commitActiveFromVisualCenter("scroll");
    };
    const handleScrollEnd = () => commitActiveFromVisualCenter("scroll");
    const handleResize = () => {
      updateEdgeSpace();
      scheduleUpdate();
      commitActiveFromVisualCenter("scroll");
    };
    const handleScrollSnapChanging = (event: Event) => commitActiveFromSnapTarget(event);

    scheduleUpdate();
    updateEdgeSpace();
    commitActiveFromVisualCenter("programmatic");
    viewport.addEventListener("scroll", handleScroll, { passive: true });
    if (supportsScrollSnapChanging) {
      viewport.addEventListener("scrollsnapchanging", handleScrollSnapChanging);
    }
    viewport.addEventListener("scrollend", handleScrollEnd);
    window.addEventListener("resize", handleResize);
    if (typeof ResizeObserver !== "undefined") {
      resizeObserver = new ResizeObserver(handleResize);
      resizeObserver.observe(viewport);
    }

    return () => {
      const sliderRoot = getSliderRoot();
      sliderRoot?.style.removeProperty("--stack-edge-space");
      track.style.removeProperty("width");
      viewport.removeEventListener("scroll", handleScroll);
      if (supportsScrollSnapChanging) {
        viewport.removeEventListener("scrollsnapchanging", handleScrollSnapChanging);
      }
      viewport.removeEventListener("scrollend", handleScrollEnd);
      window.removeEventListener("resize", handleResize);
      resizeObserver?.disconnect();
      if (frame !== 0) cancelAnimationFrame(frame);
    };
  }, [items.length]);

  React.useEffect(() => {
    if (!onActiveCardChange || activeIndex === null) return;
    if (lastEmittedActiveIndexRef.current === activeIndex) return;
    const currentIndex = activeIndex;
    const card = itemRefs.current[currentIndex];
    if (!card || lastEmittedActiveIndexRef.current === currentIndex) return;
    const source = pendingActiveSourceRef.current;
    pendingActiveSourceRef.current = "scroll";
    lastEmittedActiveIndexRef.current = currentIndex;
    onActiveCardChange(card, currentIndex, { source });
  }, [activeIndex, onActiveCardChange]);

  const handleItemClick = React.useCallback(
    (event: React.MouseEvent<HTMLElement>, index: number) => {
      if (index === activeIndex) return;
      if ((event.target as HTMLElement).closest("a, button, input, select, textarea")) return;
      pendingActiveSourceRef.current = "click";
      setActiveIndex(index);
      scrollItemIntoView(index, "smooth", "click");
    },
    [activeIndex, scrollItemIntoView],
  );

  const mergedStyle: StackSliderStyle = { ...style };
  if (typeof inactiveScale !== "undefined") mergedStyle["--stack-inactive-scale"] = inactiveScale;
  if (typeof hoverScale !== "undefined") mergedStyle["--stack-hover-scale"] = hoverScale;
  if (typeof sidePeek !== "undefined") mergedStyle["--stack-side-peek"] = sidePeek;
  if (typeof edgeTranslate !== "undefined") mergedStyle["--stack-edge-translate"] = edgeTranslate;
  if (typeof activeZ !== "undefined") mergedStyle["--stack-z-active"] = activeZ;
  if (typeof inactiveZ !== "undefined") mergedStyle["--stack-z-inactive"] = inactiveZ;
  if (typeof overlayColor !== "undefined") mergedStyle["--stack-overlay-color"] = overlayColor;
  if (typeof overlayOpacity !== "undefined") mergedStyle["--stack-overlay-opacity"] = overlayOpacity;

  return (
    <section
      className={[styles.root, className].filter(Boolean).join(" ")}
      data-ready={isReady ? "true" : "false"}
      style={mergedStyle}
      {...props}
    >
      <div ref={viewportRef} className={[styles.viewport, viewportClassName].filter(Boolean).join(" ")}>
        <ol ref={trackRef} className={styles.track}>
          {items.map((child, index) => {
            const isCardClickable = isReady && activeIndex !== index;
            return (
              <li
                key={index}
                ref={(node) => { itemRefs.current[index] = node; }}
                className={[styles.item, itemClassName].filter(Boolean).join(" ")}
                data-active={activeIndex === index ? "true" : "false"}
              >
                <div
                  className={styles.itemInner}
                  data-clickable={isCardClickable ? "true" : "false"}
                  data-hover-capable={isHoverCapable ? "true" : "false"}
                  ref={(node) => { itemInnerRefs.current[index] = node; }}
                  onClick={(event) => handleItemClick(event, index)}
                >
                  {child}
                  <div
                    aria-hidden="true"
                    className={styles.overlay}
                    ref={(node) => { overlayRefs.current[index] = node; }}
                  />
                </div>
              </li>
            );
          })}
        </ol>
      </div>
    </section>
  );
});

CardStackSlider.displayName = "CardStackSlider";
