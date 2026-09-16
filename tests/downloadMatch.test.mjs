import test from "node:test";
import assert from "node:assert/strict";
import { filterAndRankDownloadResults, rankResultMatch } from "../src/downloadMatch.ts";

const candidate = (filename, username = "uploader", provider = "yt-dlp", id = filename) => ({
  id, provider, username, filename, file_size: 1000, bitrate: 320,
  sample_rate: 44100, format: "mp3", slots_free: true, speed_bps: 1000,
});

test("keeps every provider result while placing stronger matches first", () => {
  const unrelated = candidate("Kato Box - Pingu goes to the Pokémon Center.mp3", "Kato Box");
  const exact = candidate("Kato - Pokémon Center.mp3", "Kato");
  const archive = candidate("Pokémon Center - Kato.flac", "Internet Archive", "internet-archive");
  const audius = candidate("Pokémon Center (live).mp3", "Kato", "audius");
  const soulseek = candidate("Kato/Album/01 - Pokémon Center.flac", "peer", "soulseek");
  const source = candidate("Alternate title.mp3", "artist", "yt-dlp", "ytdlp_stream_mp3_video123");
  const input = [unrelated, source, audius, soulseek, archive, exact];
  const ranked = filterAndRankDownloadResults(input, "Kato", "Pokémon Center");
  assert.equal(ranked.length, input.length);
  assert.deepEqual(new Set(ranked.map((result) => result.id)), new Set(input.map((result) => result.id)));
  assert.ok(ranked.indexOf(exact) < ranked.indexOf(unrelated));
  assert.ok(rankResultMatch(exact, "Kato", "Pokemon Center") > rankResultMatch(unrelated, "Kato", "Pokemon Center"));
  assert.deepEqual(input.map((result) => result.id), [unrelated, source, audius, soulseek, archive, exact].map((result) => result.id));
});

test("artist-only browsing keeps provider order", () => {
  const results = [candidate("Kato - First Song.mp3"), candidate("Kato - Second Song.mp3")];
  assert.deepEqual(filterAndRankDownloadResults(results, "Kato", ""), results);
});

test("a weak or differently named source stays available for manual review", () => {
  const unrelated = candidate("Kato's World - Walmart Pokemon Cards Restock!!.mp3", "Kato's World");
  assert.deepEqual(filterAndRankDownloadResults([unrelated], "Kato", "Pokémon Center"), [unrelated]);
});
