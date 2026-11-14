import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (p) => readFileSync(new URL(`../${p}`, import.meta.url), "utf8");

test("Library has separate Local and Downloads tabs in both layouts; archive heading unchanged", () => {
  const nav = read("src/presentation/BrutalistNavigation.tsx");
  assert.match(nav, /\["library", "Local"\]/);
  assert.match(nav, /\["downloads", "Downloads"\]/);
  assert.match(nav, /\["albums", "Library"\]/);
  assert.match(nav, /ARCHIVE\./);
  assert.match(nav, /"library", "downloads", "playlists"/, "the Downloads tab stays in the Library chapter");
  const spatial = read("src/presentation/SpatialUtilities.tsx");
  assert.match(spatial, /\["library", "Local"\]/);
  assert.match(spatial, /\["downloads", "Downloads"\]/);
});

test("library tracks view has no indexed/loaded text and offers play all + shuffle", () => {
  const view = read("src/components/views/LibraryView.tsx");
  assert.doesNotMatch(view, /indexed/);
  assert.doesNotMatch(view, /loaded of/);
  assert.match(view, /onPlayAll/);
  assert.match(view, /onToggleShuffle/);
  // Header and rows share one grid template with a column gap (no overlap).
  assert.equal((view.match(/gridTemplateColumns: libraryGridColumns\(downloadControls\)/g) ?? []).length, 2);
  // The actions column is wider when the download button adds a fourth icon.
  assert.match(view, /downloadControls \? 164 : 120/);
  assert.match(view, /columnGap: "12px"/);
});

test("library playback dispatches ordered PlayTrackList", () => {
  const app = read("src/hooks/app/useLibrary.ts");
  assert.match(app, /command: "PlayTrackList"/);
  // Search filters the view only: the list always plays the full library.
  const start = app.slice(app.indexOf("const startLibraryList"), app.indexOf("const handleLibraryRowPlay"));
  assert.match(start, /track_ids: null/);
  assert.doesNotMatch(start, /tracks\.map/);
});

test("Play all toggles pause/resume and shuffle is a per-section toggle", () => {
  const view = read("src/components/views/LibraryView.tsx");
  assert.match(view, /listState === "playing" \? "Pause"/);
  assert.match(view, /aria-pressed=\{shuffleEnabled\}/);
  const app = read("src/hooks/app/useLibrary.ts");
  // Section-local shuffle state: never derived from the queue-wide is_shuffled flag.
  assert.match(app, /const \[libraryShuffle, setLibraryShuffle\] = useState<boolean>\(/);
  // Persisted across restarts under a section-specific key; Local keeps its original key.
  assert.match(app, /scope === "local" \? DOWNLOADS_SHUFFLE_KEY : "kaze\.kazeDownloads\.shuffle"/);
  assert.match(app, /localStorage\.getItem\(shuffleKey\) === "1"/);
  assert.match(app, /localStorage\.setItem\(shuffleKey/);
  assert.match(app, /command: "SetArchiveShuffle"/);
  assert.match(app, /playbackState\.source === "archive"/);
  // Each section only reports its own list as active, and plays only its own songs.
  assert.match(app, /\(activeListScope \?\? "local"\) === scope/);
  assert.match(app, /shuffle: libraryShuffle,\n\s+scope,/);
  assert.match(read("src/hooks/useScopedTracks.ts"), /online:\/\//);
  assert.match(view, /key=\{searchQuery\}/);
});

test("download controls follow only Kaze's own downloads; local songs have none", () => {
  // No fuzzy local matching feeds download state anywhere.
  for (const file of [
    "src/components/views/OnlineSearchResultsSection.tsx", "src/components/views/searchResults/AllResultsTab.tsx", "src/components/views/searchResults/SongsTab.tsx",
    "src/components/views/DiscoveryView.tsx", "src/components/views/discovery/DiscoveryTrackCard.tsx",
    "src/components/views/CollectionDetailView.tsx", "src/components/views/collection/CollectionTracks.tsx",
  ])
    assert.doesNotMatch(read(file), /downloaded=\{!!\w+\.local_copy\}|downloaded=\{!!track\.is_downloaded\}/, file);
  assert.match(read("src/presentation/BrutalistDeck.tsx"), /track && \(kind !== false \|\| kazeDownload\) && <DownloadButton/);
  assert.match(read("src/components/views/collection/CollectionTracks.tsx"), /!track\.rawLocalTrack && <DownloadButton/);
  assert.match(read("src/components/views/discovery/DiscoveryTrackCard.tsx"), /!isLocalTrack && <DownloadButton/);
});

test("Local and Downloads lists request their own section from the backend", () => {
  const library = read("src/hooks/app/useLibrary.ts");
  assert.match(library, /useLibraryPage<Track>\("GetLibraryTracksPage", \{ size: 200, [^\n]*scope: "local" \}\)/);
  assert.match(library, /useLibraryPage<Album>\("GetLibraryAlbumsPage", \{ size: 100, [^\n]*scope: "local" \}\)/);
  assert.match(read("src/hooks/app/useDownloadsSection.ts"), /useLibraryPage<Track>\("GetLibraryTracksPage", \{ size: 200, [^\n]*scope: "downloads" \}\)/);
  assert.match(read("src/hooks/useLibraryPage.ts"), /\.\.\.\(scope \? \{ scope \} : \{\}\)/);
  // Search/album rows: Local leaves out Kaze's folder, Downloads keeps only it.
  assert.match(read("src/hooks/useScopedTracks.ts"), /\.filter\(\(t: Track\) => inSection\(t, downloadsDir, section\)\)/);
  assert.match(read("src/hooks/app/useDownloadsSection.ts"), /useScopedTracks\(\{ account: userId, query, active: searching, downloadsDir, section: "downloads" \}\)/);
  // The folder watcher's event refreshes everything live.
  assert.match(read("src/hooks/app/backendEvents/libraryEvents.ts"), /case "DownloadsChanged":/);
});

test("downloaded songs can be removed from the deck, the bar and the Downloads list", () => {
  const app = read("src/App.tsx");
  assert.match(app, /<ConfirmModal \/>/, "one store-driven popup, no App state");
  assert.doesNotMatch(app, /useConfirmRequest/, "App never subscribes to the popup");
  assert.match(read("src/components/layout/appViews/otherViews.tsx"), /downloadControls\s+emptyTitle="No downloaded songs yet"/);
  assert.match(read("src/components/views/LibraryView.tsx"), /downloadControls && <DownloadButton/);
  assert.match(read("src/presentation/BrutalistDeck.tsx"), /kind !== false \|\| kazeDownload/);
  assert.match(read("src/components/NowPlayingBar.tsx"), /isOnline \|\| isKazeDownload/);
  assert.match(read("src/hooks/app/backendEvents/libraryEvents.ts"), /case "DownloadsChanged":[\s\S]{0,200}fetchTrackPlaylistMemberships\(\)/);
});

test("a search splits songs between Local and Downloads by Kaze's folder", async () => {
  const { inSection } = await import("../src/downloadsFolder.ts");
  const dir = "/home/u/Music/Kaze/";
  const kaze = { file_path: "/home/u/Music/Kaze/Artist - Song (abc).m4a" };
  const mine = { file_path: "/home/u/Music/Kaze Old/Song.flac" };
  assert.equal(inSection(kaze, dir, "downloads"), true);
  assert.equal(inSection(kaze, dir, "local"), false);
  assert.equal(inSection(mine, dir, "local"), true, "a sibling folder with the same prefix stays Local");
  assert.equal(inSection(mine, dir, "downloads"), false);
});

test("the Downloads tab is the Local list for Kaze's songs, with no transfers table or search form", () => {
  const views = read("src/components/layout/appViews/otherViews.tsx");
  const route = views.slice(views.indexOf("  downloads: (c) =>"), views.indexOf("  wishlist: (c) =>"));
  assert.match(route, /^\s+<LibraryView/m);
  for (const prop of ["onSearch={c.handleSearchLibrary}", "searchQuery={c.destination.libraryQuery}", "onRescan=", "onPlayAll={c.kazeDownloadsPlayback.handleLibraryPlayAll}"])
    assert.ok(route.includes(prop), prop);
  assert.doesNotMatch(views, /DownloadsView|Transfers|Find a song to download/);
});
