import { Disc } from "lucide-react";

const fill = { width: "100%", height: "100%" } as const;

/** Cover art filling its box, or a centred disc icon when there is none. */
export function CoverOrDisc({ src, alt, iconSize, background = "var(--overlay-5)" }: {
  src?: string | null; alt: string; iconSize: number; background?: string;
}) {
  if (src) return <img decoding="async" src={src} alt={alt} style={{ ...fill, objectFit: "cover" }} />;
  return (
    <div style={{ ...fill, display: "flex", alignItems: "center", justifyContent: "center", background }}>
      <Disc size={iconSize} color="var(--accent-light)" />
    </div>
  );
}
