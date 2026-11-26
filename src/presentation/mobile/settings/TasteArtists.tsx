import { Check, Search } from "lucide-react";
import { useTasteArtistPicks, useArtistSearch } from "./useTasteArtistPicks";
import { ARTIST_LIMIT } from "./useMusicTaste";
import { mergeArtistChoices } from "./artistChoices";

export function TasteArtists({ languages, selected, disabled, onToggle }: {
  languages: string[];
  selected: string[];
  disabled: boolean;
  onToggle: (name: string) => void;
}) {
  const { search, setSearch, suggestions, searchError } = useArtistSearch();
  const { pool, related, loading } = useTasteArtistPicks(languages, selected[selected.length - 1]);
  const choices = mergeArtistChoices(selected, search.trim() ? suggestions : [...related, ...pool]);
  return <>
    <p className="m-taste__intro">Choose up to five artists. Tap an artist to see more like them.</p>
    <label className="m-field m-field--sm m-taste__search">
      <Search size={18} strokeWidth={2} className="m-field__icon" />
      <input type="search" className="m-field__input" placeholder="Search artists…" value={search} disabled={disabled}
        onChange={(e) => setSearch(e.target.value)} aria-label="Search artists" autoComplete="off" />
    </label>
    <div className="m-taste__label"><span>ARTISTS</span><span className="m-taste__count">{selected.length} / {ARTIST_LIMIT}</span></div>
    <div className="m-taste__grid" role="group" aria-label="Artists">
      {choices.map((name) => {
        const on = selected.includes(name);
        return <button key={name} type="button" className="m-taste-artist" aria-pressed={on}
          disabled={disabled || (!on && selected.length >= ARTIST_LIMIT)} onClick={() => onToggle(name)}>
          <span className="m-taste-artist__badge" aria-hidden="true">{on ? <Check size={20} /> : name.trim().charAt(0).toUpperCase()}</span>
          <span>{name}</span>
        </button>;
      })}
    </div>
    {!choices.length && <p className="m-set-note">{loading && !search ? "Loading artists…" : "Search for an artist to add them."}</p>}
    {searchError && <p className="m-set-note is-error" role="alert">{searchError}</p>}
  </>;
}
