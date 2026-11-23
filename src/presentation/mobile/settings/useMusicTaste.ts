import { useEffect, useRef, useState } from "react";
import { dispatchCommand, executeQuery } from "../../../services/api";
import { sameSongMetadata } from "../../../localTrackMatch";
import { useDebouncedValue } from "../../useDebouncedValue";
import type { RecommendationSetup } from "../../../types";

export const TASTE_GENRES = ["Pop", "Rock", "Hip-Hop", "R&B", "Electronic", "Indie", "Jazz", "Classical", "Country", "Folk", "Metal", "Devotional"];
export const TASTE_LIMIT = 5;

/** The account's saved artists and genres (skipped when the caller already has them). */
function useSavedTaste(userId: string, initial: RecommendationSetup | undefined) {
  const [artists, setArtists] = useState<string[]>(initial?.artists || []);
  const [genres, setGenres] = useState<string[]>(initial?.genres || []);
  const [loading, setLoading] = useState(!initial);
  const [loadError, setLoadError] = useState("");
  useEffect(() => {
    if (initial) return;
    let active = true;
    executeQuery({ query: "GetRecommendationSetup" }).then((result) => {
      if (!active) return;
      if (result.type !== "RecommendationSetup" || result.data?.user_id !== userId) throw Error("unavailable");
      setArtists(result.data.artists);
      setGenres(result.data.genres);
    }).catch(() => { if (active) setLoadError("Couldn’t load your music taste. Open it again to retry."); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [initial, userId]);
  return { artists, setArtists, genres, setGenres, loading, loadError };
}

const uniqueArtists = (data: unknown): string[] => {
  const names = Array.isArray(data) ? data.flatMap((s: { artist?: string }) => (s.artist ? [s.artist] : [])) : [];
  return names.filter((n, i) => names.findIndex((o) => sameSongMetadata("artist", n, "artist", o)) === i).slice(0, 12);
};

/** Artist names from an online search, 250ms after typing stops. */
function useArtistSearch() {
  const [search, setSearch] = useState("");
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [searchError, setSearchError] = useState("");
  const needle = useDebouncedValue(search).trim();
  useEffect(() => {
    let active = true;
    setSuggestions([]);
    setSearchError("");
    if (needle.length < 2) return;
    executeQuery({ query: "SearchOnlineMusic", payload: { query: needle, limit: 25 } })
      .then((result) => { if (active) setSuggestions(uniqueArtists(result.data)); })
      .catch(() => { if (active) setSearchError("Artist search is unavailable right now. You can still pick genres."); });
    return () => { active = false; };
  }, [needle]);
  return { search, setSearch, suggestions, searchError };
}

const toggleIn = (value: string, values: string[]) =>
  values.includes(value) ? values.filter((v) => v !== value) : values.length < TASTE_LIMIT ? [...values, value] : values;

/** Loads, edits and saves the account's music taste (same queries as the PC Music Taste). */
export function useMusicTaste(userId: string, initial: RecommendationSetup | undefined, onSaved: () => void) {
  const saved = useSavedTaste(userId, initial);
  const finder = useArtistSearch();
  const [busy, setBusy] = useState(false);
  const [saveError, setSaveError] = useState("");
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const { artists, genres } = saved;

  const save = async (skipped: boolean) => {
    if (busy) return;
    setBusy(true);
    setSaveError("");
    try {
      await dispatchCommand({ command: "SaveRecommendationSetup", payload: { expected_user_id: userId, artists, genres, skipped } });
      if (alive.current) onSaved();
    } catch {
      if (alive.current) setSaveError("Couldn’t save your music taste. Try again.");
    } finally {
      if (alive.current) setBusy(false);
    }
  };

  return {
    artists, genres, busy, save,
    loading: saved.loading,
    error: saveError || saved.loadError || finder.searchError,
    search: finder.search,
    setSearch: finder.setSearch,
    suggestions: finder.suggestions.filter((n) => !artists.some((a) => sameSongMetadata("artist", a, "artist", n))),
    toggleArtist: (name: string) => saved.setArtists(toggleIn(name, artists)),
    toggleGenre: (genre: string) => saved.setGenres(toggleIn(genre, genres)),
  };
}
