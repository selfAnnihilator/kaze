import { useRef, useState, type ReactNode } from "react";
import { ChevronLeft, ChevronRight, Play } from "lucide-react";
import { PullIndicator } from "../pullRefresh/PullIndicator";
import { PullView } from "../pullRefresh/pullView";
import { scrollerOf } from "../pullRefresh/usePullToRefresh";
import { DownloadButton } from "../../../components/DownloadButton";
import { playlistCollection } from "../playlistCollection";
import { SectionHead } from "../SectionHead";
import { Sleeve } from "../Sleeve";
import { TrackRow } from "../TrackRow";
import { joinMeta } from "../format";
import { openTrackSheet } from "../sheets/trackSheet";
import { DiscoverCard } from "./DiscoverCard";
import { TopCharts } from "./TopCharts";
import { useChapterSwipe } from "./useChapterSwipe";
import { clampChapter } from "./swipeStep";
import { useDiscoveryMemory } from "../../ViewMemory";
import type { DiscoveryRecommendation, Playlist } from "../../../types";
import type { MobileScreenProps } from "../types";

type Ctx = MobileScreenProps["c"];
const TOTAL = 4;
const pad = (n: number) => String(n).padStart(2, "0");
const recKey = (rec: DiscoveryRecommendation, idx: number) => rec.external_track_id || `${rec.title}:${idx}`;

/** Chapter band (Figma C1–C3): "CH.0N //", title, previous, "0N/04", next (pull down refreshes). */
function ChapterHead({ n, title, onJump }: { n: number; title: string; onJump: (to: number) => void }) {
  return (
    <SectionHead kicker={`CH.${pad(n)} //`} title={title} compact actions={<>
      <button type="button" className="m-sechead__btn" disabled={n === 1} onClick={() => onJump(n - 1)} aria-label="Previous chapter"><ChevronLeft size={20} strokeWidth={2} /></button>
      <span className="m-sechead__page">{pad(n)}/{pad(TOTAL)}</span>
      <button type="button" className="m-sechead__btn" disabled={n === TOTAL} onClick={() => onJump(n + 1)} aria-label="Next chapter"><ChevronRight size={20} strokeWidth={2} /></button>
    </>} />
  );
}

/** For You song (Figma C2): sleeve, title, artist · album, download, ⋮. */
export function ForYouRow({ rec, c }: { rec: DiscoveryRecommendation; c: Ctx }) {
  const more = () => openTrackSheet("actions", {
    id: rec.matched_local_track_id || rec.external_track_id, title: rec.title, artist: rec.artist, album: rec.album,
    cover: rec.cover_art_url, online: !rec.matched_local_track_id, rec,
  });
  return (
    <TrackRow noIndex title={rec.title} subtitle={joinMeta(rec.artist, rec.album)} src={rec.cover_art_url} trackId={rec.matched_local_track_id}
      active={!!rec.matched_local_track_id && rec.matched_local_track_id === c.activePlayingTrackId}
      onPress={() => void c.handlePlayOnlineTrack(rec)} onMore={more}
      trailing={rec.matched_local_track_id ? undefined : (
        <span className="m-discover-card__download"><DownloadButton trackId={rec.external_track_id} sourceId={rec.playback_source?.track_id} title={rec.title} compact /></span>
      )} />
  );
}

/** Mix card (Figma C2): wash card, sleeve, name, tracks, then "MIX" and a red play square. */
function MixCard({ mix, c }: { mix: Playlist; c: Ctx }) {
  return (
    <article className="m-dmix">
      <Sleeve size="mixcard" src={mix.cover_art_url} trackId={mix.first_track_id} title={mix.name} onPress={() => c.handleOpenCollection(playlistCollection(mix))} />
      <strong className="m-dmix__name">{mix.name}</strong>
      <span className="m-profile-meta">{joinMeta(`${mix.track_count ?? 0} TRACKS`, mix.mix_type?.toUpperCase())}</span>
      <div className="m-dmix__foot">
        <span className="m-kicker">MIX</span>
        <button type="button" className="m-dmix__play" onClick={() => void c.handlePlayPlaylist(mix.id)} aria-label={`Play ${mix.name}`}><Play size={18} strokeWidth={2} /></button>
      </div>
    </article>
  );
}

function Empty({ children }: { children: ReactNode }) {
  return <p className="m-empty-msg">{children}</p>;
}

function Row({ children, empty }: { children: ReactNode[]; empty: string }) {
  return children.length ? <div className="m-discover-cards-row">{children}</div> : <Empty>{empty}</Empty>;
}

/**
 * Discover's four chapters (Figma C1 Trending, C2 For You and Mixes For You,
 * C3 Top Charts), one at a time like the PC: it stays in place, and a swipe up
 * or › goes to the next chapter, a swipe down or ‹ to the previous one. Each
 * chapter is one row that scrolls sideways.
 */
export function DiscoverChapters({ c }: { c: Ctx }) {
  const memory = useDiscoveryMemory();
  const [active, setActive] = useState(() => clampChapter(memory.chapter, TOTAL));
  const [dir, setDir] = useState<1 | -1>(1);
  const [chartsKey, setChartsKey] = useState(0);
  const stage = useRef<HTMLDivElement>(null);
  const go = (to: number) => {
    const next = clampChapter(to, TOTAL);
    if (next === active) return;
    setDir(next > active ? 1 : -1);
    setActive(next);
    memory.chapter = next;
  };
  const pullEl = useRef<HTMLDivElement>(null);
  // A long slow pull from the top refreshes the chapter on screen.
  const refreshers: (() => Promise<unknown>)[] = [
    () => c.refreshTrending(),
    () => c.refreshPersonalRecommendations(),
    () => c.handleRefreshPlaylists(),
    async () => setChartsKey((k) => k + 1),
  ];
  useChapterSwipe(stage, (step) => go(active + step), {
    view: () => (pullEl.current ? new PullView(pullEl.current) : null),
    atTop: () => { const el = pullEl.current; return !!el && (scrollerOf(el)?.scrollTop ?? 0) <= 0; },
    refresh: () => refreshers[active](),
  });
  const jump = (n: number) => go(n - 1);

  const trending = (c.worldTrending.length ? c.worldTrending : c.discoveryRecs).slice(0, 12);
  const forYou = c.discoveryRecs.slice(0, 20);
  const seen = new Set<string>();
  const mixes = c.playlists.filter((p) => p.is_smart_mix === 1 && !seen.has(p.name) && seen.add(p.name));
  const chapters = [
    <><ChapterHead n={1} title="TRENDING SONGS" onJump={jump} />
      <Row empty="Nothing trending right now. Pull down to try again.">{trending.map((rec, i) => <DiscoverCard key={recKey(rec, i)} rec={rec} index={i + 1} c={c} />)}</Row></>,
    <><ChapterHead n={2} title="FOR YOU" onJump={jump} />
      <Row empty="No picks yet. Play a few songs so Kaze can learn your taste.">{forYou.map((rec, i) => <DiscoverCard key={recKey(rec, i)} rec={rec} index={i + 1} c={c} />)}</Row></>,
    <><ChapterHead n={3} title="MIXES FOR YOU" onJump={jump} />
      {mixes.length ? <div className="m-dmix-row">{mixes.map((mix) => <MixCard key={mix.id} mix={mix} c={c} />)}</div>
        : <Empty>No mixes yet. Kaze builds them from what you play.</Empty>}</>,
    <><ChapterHead n={4} title="TOP CHARTS" onJump={jump} />
      <TopCharts c={c} revision={chartsKey} /></>,
  ];
  return (
    <div ref={stage} className="m-discover-stage">
      <PullIndicator ref={pullEl} />
      <section key={active} className={`m-discover-chapter ${dir > 0 ? "is-from-below" : "is-from-above"}`} aria-label={`Chapter ${active + 1} of ${TOTAL}`}>
        {chapters[active]}
      </section>
    </div>
  );
}
