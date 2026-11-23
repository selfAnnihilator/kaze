import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { hoursMinutes } from "../src/presentation/mobile/profile/useProfileStats.ts";
import { formatBytes, userInitials } from "../src/presentation/mobile/format.ts";
import { closeTrackSheet, openTrackSheet } from "../src/presentation/mobile/sheets/trackSheet.ts";
import { swipeStep, clampChapter } from "../src/presentation/mobile/discover/swipeStep.ts";
import { shownArtwork } from "../src/services/shownArtwork.ts";
import { dragStarted, shouldOpen } from "../src/presentation/mobile/deck/dragMath.ts";

const read = (f) => readFileSync(new URL(`../${f}`, import.meta.url), "utf8");

test("profile times read like Figma F1 (1H 52M, 48M)", () => {
  assert.equal(hoursMinutes(6720), "1H 52M");
  assert.equal(hoursMinutes(2880), "48M");
  assert.equal(hoursMinutes(-5), "0M");
  assert.equal(hoursMinutes(386 * 3600 + 1800), "386H", "100+ hours drop the minutes so a year fits its box");
});

test("avatar initials and storage sizes", () => {
  assert.equal(userInitials("Gautam Lal"), "GL");
  assert.equal(userInitials("kaze_user"), "KU");
  assert.equal(formatBytes(1.24 * 1024 ** 3), "1.24 GB");
  assert.equal(formatBytes(23.6 * 1024 ** 2), "23.6 MB");
});

test("track sheets open and close without App state", () => {
  openTrackSheet("actions", { id: "t1", title: "Song" });
  openTrackSheet("not_interested", { id: "t1", title: "Song" });
  closeTrackSheet();
  assert.doesNotMatch(read("src/App.tsx"), /useTrackSheet/);
});

test("the phone draws only Figma screens, never the PC views or dialogs", () => {
  const overlays = read("src/presentation/mobile/MobileOverlays.tsx");
  for (const pc of ["AddToPlaylistModal\"", "OnboardingModal", "RecommendationSetupGate", "AuthModal\""]) {
    assert.doesNotMatch(overlays, new RegExp(`from ".*${pc}`), pc);
  }
  const pages = read("src/presentation/mobile/MobilePageView.tsx");
  assert.doesNotMatch(pages, /OTHER_VIEWS|LIBRARY_VIEWS|LyricsView/);
  assert.doesNotMatch(pages, /wishlist/i, "Wishlist was removed from the app");
  assert.doesNotMatch(read("src/presentation/mobile/MobileSettings.tsx"), /from "\.\.\/\.\.\/components/);
  assert.doesNotMatch(read("src/presentation/mobile/DeckParts.tsx"), /NotInterestedButton/);
});

test("the phone opens on the A1 startup screen, the PC keeps its intro", () => {
  const main = read("src/main.tsx");
  assert.match(main, /phone \? <MobileStartup \/> : <StartupScreen \/>/);
  assert.match(main, /if \(!phone\) await playIntro/);
});

test("new lists stay windowed and nothing animates layout", () => {
  assert.match(read("src/presentation/mobile/MobileLyrics.tsx"), /WindowedList/);
  for (const f of ["mobile-settings.css", "mobile-profile.css", "mobile-lyrics.css", "mobile-startup.css"]) {
    const css = read(`src/presentation/mobile/${f}`);
    assert.doesNotMatch(css, /backdrop-filter|filter:\s*blur/, f);
    assert.doesNotMatch(css, /transition:[^;]*(width|height|top|left)/, f);
  }
});

test("Discover stacks the four Figma chapters and has its search tabs and offline screen", () => {
  const ch = read("src/presentation/mobile/discover/DiscoverChapters.tsx");
  for (const title of ["TRENDING SONGS", "FOR YOU", "MIXES FOR YOU", "TOP CHARTS"]) assert.match(ch, new RegExp(`"${title}"`), title);
  assert.match(read("src/presentation/mobile/discover/useCharts.ts"), /"GetCharts"/);
  const search = read("src/presentation/mobile/discover/SearchResults.tsx");
  for (const tab of ["ALL", "SONGS", "ARTISTS", "ALBUMS", "PLAYLISTS"]) assert.match(search, new RegExp(`label: "${tab}"`), tab);
  assert.doesNotMatch(read("src/presentation/mobile/discover/TopCharts.tsx"), /BY COUNTRY/, "filters come from the charts");
  assert.match(read("src/presentation/mobile/discover/useCharts.ts"), /export function chartFilters/);
  const offline = read("src/presentation/mobile/discover/Offline.tsx");
  assert.match(offline, /NO SIGNAL\. STILL MUSIC\./);
  assert.match(offline, /openLibrary\("downloads"\)/);
});

test("playlists can be imported, renamed and deleted from the phone", () => {
  assert.match(read("src/presentation/mobile/LibraryPanes.tsx"), /<ImportPlaylistSheet/);
  const sheet = read("src/presentation/mobile/playlists/ImportPlaylistSheet.tsx");
  assert.match(sheet, /handleInspectJioSaavnPlaylist/);
  assert.match(sheet, /handleSaveImportedPlaylist/);
  assert.doesNotMatch(sheet, /wishlist/i, "Wishlist was removed from the app");
  const menu = read("src/presentation/mobile/playlists/PlaylistOptions.tsx");
  assert.match(menu, /handleRenamePlaylist/);
  assert.match(menu, /confirmAction\(/);
  assert.match(read("src/presentation/mobile/CollectionPane.tsx"), /<PlaylistOptions/);
});

test("the phone shows G10 notices, not the PC toasts and banners", () => {
  const overlays = read("src/presentation/mobile/MobileOverlays.tsx");
  for (const pc of ["ToastContainer", "UpdateBanner", "PlaybackNotice", "RecommendationNotice"]) {
    assert.doesNotMatch(overlays, new RegExp(`from "\\.\\./\\.\\./components/.*${pc}"`), pc);
  }
  for (const f of ["mobile-notices.css", "mobile-discover.css", "mobile-library.css"]) {
    const css = read(`src/presentation/mobile/${f}`);
    assert.doesNotMatch(css, /backdrop-filter|filter:\s*blur/, f);
    assert.doesNotMatch(css, /transition:[^;]*(width|height|top|left)/, f);
  }
});

test("Why this track is gone from the phone and the PC", () => {
  for (const f of ["src/App.tsx", "src/components/NowPlayingBar.tsx", "src/components/layout/AppOverlays.tsx", "src/hooks/app/useFlow.ts",
    "src/presentation/mobile/HomeSections.tsx", "src/presentation/mobile/DeckParts.tsx", "src/presentation/mobile/sheets/TrackSheets.tsx",
    "src/presentation/mobile/MobileOverlays.tsx"]) {
    assert.doesNotMatch(read(f), /WhyTrack|whyTrack|InspectWhy|Why this track/, f);
  }
});

test("toasts carry UNDO and RETRY, and the update banner keeps its UPDATE button", () => {
  assert.match(read("src/types.ts"), /action\?: ToastAction/);
  assert.match(read("src/hooks/app/backendEvents/libraryEvents.ts"), /retryAction\(event\.payload\?\.task_id\)/);
  const phone = read("src/presentation/mobile/phoneToasts.ts");
  assert.match(phone, /label: "UNDO", run: \(\) => c\.handleDequeueTrack/);
  const notices = read("src/presentation/mobile/MobileNotices.tsx");
  assert.match(notices, /toast\.action\.label/);
  assert.match(notices, />UPDATE</);
});

test("profile lists show every entry inside a fixed-height scroll", () => {
  const lists = read("src/presentation/mobile/profile/ProfileTopLists.tsx");
  assert.doesNotMatch(lists, /top_songs\.slice|top_artists\.slice|top_genres \?\? \[\]\)\.slice/);
  assert.match(lists, /m-profile-scroll--songs/);
  assert.match(read("src/presentation/mobile/mobile-profile.css"), /\.m-profile-scroll--songs \{ max-height: calc\(4 \* 59/);
});

test("stats are for signed-in users: guests get the sign-in sheet from the avatar", () => {
  const shell = read("src/presentation/mobile/useMobileShell.ts");
  assert.match(shell, /openProfile: \(\) => \(initials \? showPage\("profile"\) : setIsAuthModalOpen\(true\)\)/);
  assert.match(shell, /if \(!currentUser && page === "profile"\) layers\.back\(\)/);
});

test("no sparkle (star) icons on the phone", () => {
  for (const f of ["MobileHome", "MobileSettings", "HomeSections", "DeckParts", "sheets/TrackSheets"]) {
    assert.doesNotMatch(read(`src/presentation/mobile/${f}.tsx`), /Sparkles/, f);
  }
});

test("tapping a graph bar sums that day and its Monday–Sunday week inside the month", async () => {
  const { dayFocus } = await import("../src/presentation/mobile/profile/dayFocus.ts");
  // November 2026: the 1st is a Sunday, the 9th a Monday, the 30th a Monday.
  const graph = Array.from({ length: 30 }, (_, i) => ({ day: i + 1, date: `2026-11-${String(i + 1).padStart(2, "0")}`, total_seconds: (i + 1) * 60 }));
  assert.deepEqual(dayFocus(graph, "2026-11-14"), { dayLabel: "NOV 14", daySeconds: 840, weekLabel: "NOV 9–15", weekSeconds: (9 + 10 + 11 + 12 + 13 + 14 + 15) * 60 });
  assert.equal(dayFocus(graph, "2026-11-01").weekLabel, "NOV 1", "a week cut by the month edge names only its days");
  assert.equal(dayFocus(graph, "2026-11-30").weekSeconds, 30 * 60);
  assert.equal(dayFocus(graph, "2026-12-01"), null);
  const css = read("src/presentation/mobile/mobile-profile.css");
  assert.doesNotMatch(css, /transition:[^;]*(width|height|top|left)/);
});

test("profile shows This year and This month above Today and This week", () => {
  const src = read("src/presentation/mobile/profile/ProfileStats.tsx");
  const order = ["THIS YEAR", "THIS MONTH", "TODAY", "THIS WEEK"].map((l) => src.indexOf(`"${l}"`));
  assert.ok(order.every((i) => i > 0) && order.every((i, k) => k === 0 || i > order[k - 1]), "year, month, today, week");
  assert.match(read("src/presentation/mobile/mobile-profile.css"), /\.m-profile-boxes \{ display: grid; grid-template-columns: 1fr 1fr;/);
});

test("Change Photo uses the phone's image picker and sends the image itself", () => {
  const head = read("src/presentation/mobile/profile/ProfileHead.tsx");
  assert.match(head, /type="file" accept="image\/\*"/);
  assert.match(head, /command: "UploadAvatar", payload: \{ image_base64: await shrinkPhoto\(file\) \}/);
  assert.match(read("src/services/photoUpload.ts"), /PHOTO_EDGE = 512/);
  assert.match(read("src/ipcTypes.ts"), /UploadAvatar"; payload\?: \{ file_path\?: string; image_base64\?: string \}/);
});

test("profiles reload when a cloud sync lands, on the phone and the PC", () => {
  assert.match(read("src/hooks/app/backendEvents/libraryEvents.ts"), /case "CloudDataSynced": \{\s*cloudSyncSignal\.emit\(\);/);
  assert.match(read("src/presentation/mobile/MobileProfile.tsx"), /cloudSyncSignal\.subscribe\(\(\) => void reload\(\)\)/);
  assert.match(read("src/components/views/StatsView.tsx"), /cloudSyncSignal\.subscribe\(\(\) => void loadStats\(true\)\)/);
  assert.match(read("src-tauri/src/core/processor/live_sync.rs"), /PROFILE_EVERY: Duration = Duration::from_secs\(20\)/);
});

test("PC Change Photo picks in the window and sends the image, never a backend dialog", () => {
  const view = read("src/components/views/StatsView.tsx");
  assert.match(view, /type="file" accept="image\/png,image\/jpeg,image\/webp"/);
  assert.match(view, /command: "UploadAvatar", payload: \{ image_base64: await shrinkPhoto\(file\) \}/);
  assert.doesNotMatch(view, /command: "UploadAvatar", payload: \{\} \}/);
});

test("the profile photo shows a buffer bar while a new photo saves", () => {
  assert.match(read("src/presentation/mobile/profile/ProfileHead.tsx"), /photo\.busy && <span className="m-profile-head__buffer"/);
  const css = read("src/presentation/mobile/mobile-profile.css");
  assert.match(css, /\.m-profile-head__buffer span \{[^}]*animation: m-profile-buffer/);
  assert.match(css, /@keyframes m-profile-buffer \{ from \{ transform: translateX\(0\); \} to \{ transform: translateX\(186%\); \} \}/);
  assert.match(css, /\.is-busy > img, \.m-profile-head__photo\.is-busy > svg \{ opacity: 0\.4; \}/);
});

test("the Deck's Queue button toggles the queue inside the Deck", () => {
  const deck = read("src/presentation/mobile/MobileDeck.tsx");
  assert.match(deck, /queueOpen \? \(\s*<div className="m-deck__queue m-queue"><QueueBody c=\{c\} kicker="THE DECK \/\/ QUEUE" \/><\/div>/);
  assert.match(deck, /<DeckTabs player=\{player\} deck=\{deck\} queueOpen=\{queueOpen\} onToggleQueue=\{toggleQueue\} \/>/);
  const tabs = read("src/presentation/mobile/DeckParts.tsx");
  assert.match(tabs, /m-deck__tab-item \$\{queueOpen \? "is-active" : ""\}`\} onClick=\{onToggleQueue\}/);
  assert.doesNotMatch(tabs, /goTab\("queue"\)/);
  assert.match(read("src/presentation/mobile/MobileQueue.tsx"), /<QueueBody c=\{c\} kicker="04 \/\/ QUEUE" \/>/);
});

test("Discover swipes: up is next, down is previous, sideways belongs to the rows", () => {
  assert.equal(swipeStep(4, -90), 1, "swipe up goes to the next chapter");
  assert.equal(swipeStep(-6, 120), -1, "swipe down goes to the previous chapter");
  assert.equal(swipeStep(140, -60), 0, "a mostly-sideways swipe scrolls the row");
  assert.equal(swipeStep(0, 30), 0, "a short drag does nothing");
  assert.equal(clampChapter(-1, 4), 0);
  assert.equal(clampChapter(9, 4), 3);
});

test("Discover shows one chapter at a time, each a sideways row, like the PC", () => {
  const ch = read("src/presentation/mobile/discover/DiscoverChapters.tsx");
  assert.match(ch, /<section key=\{active\} className=\{`m-discover-chapter/, "only the active chapter renders");
  assert.match(ch, /useChapterSwipe\(stage, \(step\) => go\(active \+ step\)\)/);
  assert.match(ch, /const jump = \(n: number\) => go\(n - 1\)/, "the arrows page too");
  assert.doesNotMatch(ch, /scrollIntoView/);
  assert.match(ch, /forYou\.map\(\(rec, i\) => <DiscoverCard /, "For You uses the Trending card");
  assert.match(ch, /useDiscoveryMemory\(\)/, "the chapter is remembered like on the PC");
  assert.match(read("src/presentation/mobile/discover/TopCharts.tsx"), /className="m-chart-row"/);
  const css = read("src/presentation/mobile/mobile-discover.css");
  assert.match(css, /\.m-discover-stage \{[^}]*overflow: hidden;[^}]*touch-action: pan-x;/);
  assert.match(css, /@keyframes m-chapter-in \{ from \{ opacity: 0; transform: translateY/);
});

test("Previous shows the very artwork a song had while playing, and old entries fall out", () => {
  shownArtwork.remember("song-a", "https://img/a.jpg");
  shownArtwork.remember("song-b", null);
  assert.equal(shownArtwork.get("song-a"), "https://img/a.jpg");
  assert.equal(shownArtwork.get("song-b"), undefined, "nothing shown, nothing remembered");
  for (let i = 0; i < 70; i++) shownArtwork.remember(`filler-${i}`, `https://img/${i}.jpg`);
  assert.equal(shownArtwork.get("song-a"), undefined, "the oldest entry is dropped past 64");
  assert.equal(shownArtwork.get("filler-69"), "https://img/69.jpg");
});

test("Previous, now and Next share one artwork lookup on the PC and the phone", () => {
  const details = read("src/hooks/app/useNowPlayingDetails.ts");
  assert.match(details, /export function lookupArtwork\(/);
  assert.match(details, /lookupArtwork\(track, \{ tracks, activeCollection, discoveryRecs, albums \}\)/, "now playing uses it");
  const order = ["track.cover_art_url", "sources.tracks.find", "sources.activeCollection", "listed(sources.discoveryRecs)", "listed(sources.worldTrending)", "sources.albums.find"].map((k) => details.indexOf(k));
  assert.ok(order.every((v, k) => v > 0 && (k === 0 || v > order[k - 1])), "lookup order");
  const neighbours = read("src/hooks/app/useNeighbourTracks.ts");
  assert.match(neighbours, /shownArtwork\.get\(track\.id\) \|\| lookupArtwork\(track, sources\)/);
  assert.match(read("src/App.tsx"), /useNeighbourTracks\(\{/);
  for (const f of ["src/components/layout/appViews/otherViews.tsx", "src/components/layout/appViews/libraryViews.tsx", "src/presentation/mobile/MobileHome.tsx", "src/presentation/mobile/MobileDeck.tsx"]) {
    assert.doesNotMatch(read(f), /previousTrackRef\.current/, f);
  }
  const sleeve = read("src/presentation/mobile/Sleeve.tsx");
  assert.match(sleeve, /lookupable\(trackId\) && <Artwork trackId=\{trackId\}/, "the phone sleeve asks the backend by id");
  assert.match(read("src/presentation/mobile/mobile.css"), /\.m-sleeve__artwork\.is-lookup\.music-artwork--placeholder \{ display: none; \}/);
});

test("Discover cards add a song to a playlist like the PC", () => {
  const card = read("src/presentation/mobile/discover/DiscoverCard.tsx");
  assert.match(card, /<PlaylistButton rec=\{rec\} c=\{c\} \/>/);
  assert.match(card, /c\.handleOpenAddToPlaylistModal\(\{ id, title: rec\.title/, "opens the phone's Add to playlist sheet");
  assert.match(card, /!local\.startsWith\("online:"\) && !local\.startsWith\("itunes:"\) \? local : rec\.external_track_id/, "same id rule as the PC");
  assert.match(card, /saved \? BookmarkCheck : Bookmark/, "shows when it's already in a playlist");
});

test("Deck drag: a press stays a tap until it really moves; release past halfway opens", () => {
  assert.equal(dragStarted(0, -6, "up"), false, "a small wobble is still a tap");
  assert.equal(dragStarted(2, -14, "up"), true, "a clear upward move from the mini player drags");
  assert.equal(dragStarted(30, -12, "up"), false, "a sideways move is not a drag");
  assert.equal(dragStarted(0, 14, "up"), false, "pulling down on the mini player does nothing");
  assert.equal(dragStarted(0, 14, "down"), true, "pulling down on the Deck header closes");
  assert.equal(shouldOpen(500, 800, 0), true, "past halfway opens fully");
  assert.equal(shouldOpen(300, 800, 0), false, "short of halfway collapses");
  assert.equal(shouldOpen(120, 800, 0.9), true, "a quick flick up opens");
  assert.equal(shouldOpen(700, 800, -0.9), false, "a quick flick down closes");
});

test("The Deck grows out of the mini player and folds back into it", () => {
  const app = read("src/presentation/mobile/MobileApp.tsx");
  assert.match(app, /\{sheet\.mounted && <MobileDeck /, "stays mounted while it collapses");
  assert.match(app, /useSheetDrag\(mini, "up", sheet\.drag\)/);
  assert.match(app, /useSheetDrag\(deckHeader, "down", sheet\.drag\)/, "closing drag only on the header");
  const drag = read("src/presentation/mobile/deck/useSheetDrag.ts");
  assert.ok(drag.indexOf("dragStarted(") < drag.indexOf("e.preventDefault()"), "taps are never prevented before a drag starts");
  assert.match(drag, /if \(active && start && last\) \{\s*e\.preventDefault\(\);/, "a drag's release doesn't press the button under it");
  const sheet = read("src/presentation/mobile/deck/useDeckSheet.ts");
  assert.match(sheet, /el\.style\.transform = `translate3d\(0, /, "moves by transform, not layout");
  assert.match(sheet, /mini \? mini\.getBoundingClientRect\(\)\.top/, "folds into the mini player");
  assert.doesNotMatch(sheet, /useState<number>|setTravel/, "no per-frame React state");
});

test("Phone motion uses transform and opacity only, and respects reduced motion", () => {
  const css = read("src/presentation/mobile/mobile-motion.css");
  for (const [, body] of css.matchAll(/@keyframes [\w-]+ \{([^@]*?\})\s*\}/g)) {
    assert.doesNotMatch(body, /\b(width|height|top|left|margin|padding):/, body);
  }
  assert.match(css, /transition: transform var\(--duration-fast\)/);
  assert.match(css, /prefers-reduced-motion: reduce/);
  assert.match(read("src/presentation/mobile/MobileApp.tsx"), /<div key=\{screenKey\} className=\{`m-screen__enter/);
});

test("The top bar shows the uploaded photo, and the search field has one focus border", () => {
  const bar = read("src/presentation/mobile/MobileTopBar.tsx");
  assert.match(bar, /nav\.initials && nav\.avatarUrl\) return <img className="m-topbar__avatar-img" src=\{nav\.avatarUrl\}/);
  assert.match(read("src/presentation/mobile/useMobileShell.ts"), /const avatarUrl = currentUser\?\.avatar_data_url \|\| null;/);
  const css = read("src/presentation/mobile/mobile.css");
  assert.match(css, /\.m-shell \.m-field__input:focus-visible \{ outline: none; \}/);
  assert.match(css, /\.m-field:focus-within \{ border-color: var\(--m-red\); \}/, "the outer border stays");
});

test("Flow fills the first Home screen so For You starts below the fold", () => {
  assert.match(read("src/presentation/mobile/MobileHome.tsx"), /<section className="m-flow" aria-label="Flow">/);
  const css = read("src/presentation/mobile/mobile-home.css");
  assert.match(css, /\.m-flow \{[^}]*min-height: calc\(100% \+ var\(--space-24\)\);/, "a full screen, counting the scroll padding");
  assert.match(css, /\.m-flow > \.m-flow-carousel \{ flex: 1 0 auto; \}/, "the spare height goes around the sleeve");
});
