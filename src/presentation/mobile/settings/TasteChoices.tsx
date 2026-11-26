export function TasteChoices({ label, options, selected, disabled, onToggle }: {
  label: string;
  options: readonly string[];
  selected: readonly string[];
  disabled: boolean;
  onToggle: (value: string) => void;
}) {
  const selectedNames = new Set(selected.map((name) => name.toLowerCase()));
  return (
    <>
      <div className="m-taste__label"><span>{label}</span><span className="m-taste__count">{selected.length} SELECTED</span></div>
      <div className="m-taste__grid" role="group" aria-label={label}>
        {options.map((name) => {
          const on = selectedNames.has(name.toLowerCase());
          return (
            <button key={name} type="button" className={`m-taste__chip m-taste__genre ${on ? "is-on" : ""}`}
              aria-pressed={on} disabled={disabled} onClick={() => onToggle(name)}>
              {name.toUpperCase()}
            </button>
          );
        })}
      </div>
    </>
  );
}
