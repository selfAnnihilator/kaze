/** Section header (Figma F5/F6): red number, then the name. */
export function SettingsHead({ num, title }: { num: string; title: string }) {
  return <header className="m-set-head"><span className="m-set-head__num">{num}</span><h2 className="m-set-head__title">{title}</h2></header>;
}
