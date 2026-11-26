import { useEffect, useRef, useState } from "react";
import { dispatchCommand, executeQuery } from "../../../services/api";
import type { RecommendationSetup } from "../../../types";

export const TASTE_GENRES = ["Pop", "Rock", "Hip-Hop", "R&B", "Electronic", "Indie", "Jazz", "Classical", "Country", "Folk", "Metal", "Devotional"];
export { TASTE_LANGUAGES } from "./tasteLanguages";

/** The account's saved artists and genres (skipped when the caller already has them). */
function useSavedTaste(userId: string, initial: RecommendationSetup | undefined) {
  const [artists, setArtists] = useState<string[]>(initial?.artists || []);
  const [genres, setGenres] = useState<string[]>(initial?.genres || []);
  const [languages, setLanguages] = useState<string[]>(initial?.languages || []);
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
      setLanguages(result.data.languages ?? []);
    }).catch(() => { if (active) setLoadError("Couldn’t load your music taste. Open it again to retry."); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [initial, userId]);
  return { artists, setArtists, genres, setGenres, languages, setLanguages, loading, loadError };
}

const toggleIn = (value: string, values: string[]) =>
  values.some((v) => v.toLowerCase() === value.toLowerCase())
    ? values.filter((v) => v.toLowerCase() !== value.toLowerCase())
    : [...values, value];

/** Loads, edits and saves the account's music taste (same queries as the PC Music Taste). */
export function useMusicTaste(userId: string, initial: RecommendationSetup | undefined, onSaved: () => void) {
  const saved = useSavedTaste(userId, initial);
  const [busy, setBusy] = useState(false);
  const [saveError, setSaveError] = useState("");
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const { artists, genres, languages } = saved;

  const save = async (skipped: boolean) => {
    if (busy) return;
    setBusy(true);
    setSaveError("");
    try {
      await dispatchCommand({ command: "SaveRecommendationSetup", payload: { expected_user_id: userId, artists, genres, languages, skipped } });
      if (alive.current) onSaved();
    } catch {
      if (alive.current) setSaveError("Couldn’t save your music taste. Try again.");
    } finally {
      if (alive.current) setBusy(false);
    }
  };

  return {
    artists, genres, languages, busy, save,
    loading: saved.loading,
    error: saveError || saved.loadError,
    toggleArtist: (name: string) => saved.setArtists((values) => toggleIn(name, values)),
    toggleGenre: (genre: string) => saved.setGenres((values) => toggleIn(genre, values)),
    toggleLanguage: (language: string) => saved.setLanguages((values) => toggleIn(language, values)),
  };
}
