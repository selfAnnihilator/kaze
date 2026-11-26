import { useEffect, useState } from "react";
import { executeQuery } from "../../../services/api";
import { sameSongMetadata } from "../../../localTrackMatch";
import { useDebouncedValue } from "../../useDebouncedValue";

const uniqueArtists = (data: unknown): string[] => {
  const names = Array.isArray(data) ? data.flatMap((s: { artist?: string }) => (s.artist ? [s.artist] : [])) : [];
  return names.filter((n, i) => names.findIndex((o) => sameSongMetadata("artist", n, "artist", o)) === i).slice(0, 12);
};

/** Artist names from an online search, 250ms after typing stops. */
export function useArtistSearch() {
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
      .catch(() => { if (active) setSearchError("Artist search is unavailable right now. You can still save your choices."); });
    return () => { active = false; };
  }, [needle]);
  return { search, setSearch, suggestions, searchError };
}

const fetchTasteArtists = (payload: { languages: string[]; artist?: string }): Promise<string[]> =>
  executeQuery({ query: "GetTasteArtists", payload })
    .then((res) => (Array.isArray(res.data) ? res.data : []))
    .catch(() => []);

/** Starter artists for the chosen languages. */
function useArtistPool(languages: string[]) {
  const [pool, setPool] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let active = true;
    setLoading(true);
    fetchTasteArtists({ languages }).then((names) => {
      if (!active) return;
      setPool(names);
      setLoading(false);
    });
    return () => { active = false; };
  }, [languages]);
  return { pool, loading };
}

/** Artists similar to the last pick, 200ms after it changes. */
function useRelatedArtists(languages: string[], artist?: string) {
  const [related, setRelated] = useState<string[]>([]);
  useEffect(() => {
    let active = true;
    setRelated([]);
    if (!artist) return;
    const timer = setTimeout(() => {
      fetchTasteArtists({ languages, artist }).then((names) => { if (active) setRelated(names); });
    }, 200);
    return () => { active = false; clearTimeout(timer); };
  }, [artist, languages]);
  return related;
}

export function useTasteArtistPicks(languages: string[], artist?: string) {
  const { pool, loading } = useArtistPool(languages);
  const related = useRelatedArtists(languages, artist);
  return { pool, related, loading };
}
