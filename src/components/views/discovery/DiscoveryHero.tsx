

/** The Discover welcome banner. */
export function DiscoveryHero() {
  return (
    <section className="discovery-hero" aria-label="Welcome to Kaze">
      <div className="discovery-hero-copy">
        <p className="hero-eyebrow">A DIFFERENT KIND OF FEELING</p>
        <h1>
          Let
          <br />{" "}
          the music
          <br />{" "}
          take you
          <br />{" "}
          somewhere else.
        </h1>
        <p className="hero-japanese" lang="ja">
          どこかへ、音楽と。
        </p>
      </div>
      <div className="hero-signature" aria-hidden="true">
        <span lang="ja">音で、呼吸する。</span>
      </div>
    </section>
  );
}
