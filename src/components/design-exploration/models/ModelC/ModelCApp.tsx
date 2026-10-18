import React, { useState } from "react";
import {
  Feather,
  Heart,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Quote,
  Search,
  Volume2,
  Bookmark,
} from "lucide-react";
import { ExplorationAppContext, AppViewId } from "../../types";
import { EDITORIAL_FEATURE } from "../../mockData";

export const ModelCApp: React.FC<ExplorationAppContext> = (ctx) => {
  const {
    currentView,
    setCurrentView,
    tracks,
    playlists,
    albums,
    currentTrack,
    isPlaying,
    playbackSecs,
    volume,
    isLiked,
    searchQuery,
    setSearchQuery,
    onPlayTrack,
    onTogglePlayPause,
    onNextTrack,
    onPrevTrack,
    onSeek,
    onVolumeChange,
    onToggleLike,
  } = ctx;

  const [isBookmarked, setIsBookmarked] = useState(false);
  const feature = EDITORIAL_FEATURE;
  const romanNumerals = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"];

  const formatTime = (secs: number) => {
    const mins = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${mins}:${s.toString().padStart(2, "0")}`;
  };

  const editorialTabs: Array<{ id: AppViewId; roman: string; label: string }> = [
    { id: "home", roman: "I", label: "Cover Monograph" },
    { id: "now-playing", roman: "II", label: "Vinyl Gatefold" },
    { id: "discovery", roman: "III", label: "Gazette Reviews" },
    { id: "library", roman: "IV", label: "Archival Vault" },
    { id: "albums", roman: "V", label: "Discography" },
    { id: "playlists", roman: "VI", label: "Box Sets" },
    { id: "liked", roman: "VII", label: "Private Stash" },
    { id: "stats", roman: "VIII", label: "Annual Index" },
    { id: "settings", roman: "IX", label: "Colophon" },
  ];

  return (
    <div className="model-c-fullapp">
      {/* Editorial Masthead */}
      <header className="model-c-masthead">
        <div className="masthead-chronicle-row">
          <span>ARCHIVAL ACOUSTIC GAZETTE</span>
          <span>PARIS — TOKYO — REYKJAVIK</span>
          <span>ISSUE 48 // CURATED LISTENERS</span>
        </div>

        <div className="masthead-main-title-row">
          <h1 className="masthead-brand" onClick={() => setCurrentView("home")}>
            KAZE ATELIER
          </h1>
          <div className="masthead-curator-mark">
            <Feather size={14} />
            <span>CURATED BY {feature.curator.toUpperCase()}</span>
          </div>
        </div>

        {/* Typographic Archival Navigation Strip */}
        <nav className="masthead-editorial-tabs">
          {editorialTabs.map((tab) => {
            const isActive = currentView === tab.id;
            return (
              <button
                key={tab.id}
                type="button"
                className={`editorial-tab-btn ${isActive ? "active" : ""}`}
                onClick={() => setCurrentView(tab.id)}
              >
                <span className="tab-roman">{tab.roman}.</span>
                <span className="tab-label">{tab.label}</span>
                {isActive && <span className="tab-serif-line" />}
              </button>
            );
          })}
        </nav>
      </header>

      {/* Main Editorial Canvas */}
      <main className="model-c-body">
        {/* 1. COVER MONOGRAPH & HOME */}
        {currentView === "home" && (
          <div className="editorial-cover-view">
            <div className="editorial-lead-split">
              {/* Left Column: Monograph Essay */}
              <article className="monograph-column">
                <div className="monograph-kicker">
                  <span>CURATOR'S MONOGRAPH // FRONT PAGE</span>
                  <button
                    type="button"
                    className="monograph-bookmark-btn"
                    onClick={() => setIsBookmarked(!isBookmarked)}
                    title={isBookmarked ? "Remove Bookmark" : "Bookmark Monograph"}
                  >
                    <Bookmark size={15} fill={isBookmarked ? "currentColor" : "none"} />
                  </button>
                </div>
                <h2 className="monograph-headline">{feature.title}</h2>
                <p className="monograph-standfirst">{feature.subtitle}</p>

                <div className="monograph-body-text">
                  <p className="monograph-p lead-letter">
                    {feature.essay.split("\n\n")[0]}
                  </p>
                  <p className="monograph-p">
                    {feature.essay.split("\n\n")[1]}
                  </p>
                </div>

                <div className="monograph-pull-quote">
                  <Quote size={28} className="quote-glyph" />
                  <blockquote>“{feature.quote}”</blockquote>
                  <cite>— {feature.quoteAuthor}</cite>
                </div>

                <div className="curator-signature-block">
                  <strong>{feature.curator}</strong>
                  <span>{feature.curatorRole}</span>
                </div>
              </article>

              {/* Right Column: Featured Record & Audition */}
              <aside className="monograph-record-plate">
                <div className="plate-housing">
                  <div className="plate-badge">180g HEAVYWEIGHT MASTER</div>
                  <div className="plate-artwork">
                    <img src={currentTrack.coverArt} alt={currentTrack.title} />
                    <button type="button" className="plate-audition-btn" onClick={onTogglePlayPause}>
                      {isPlaying ? <Pause size={22} fill="#141312" /> : <Play size={22} fill="#141312" />}
                      <span>{isPlaying ? "PAUSE RECORD" : "AUDITION MASTER"}</span>
                    </button>
                  </div>
                  <div className="plate-info">
                    <h3>{currentTrack.title}</h3>
                    <p>{currentTrack.artist} — {currentTrack.album}</p>
                    <div className="plate-specs">
                      <span>{currentTrack.bpm} BPM</span>
                      <span>•</span>
                      <span>{currentTrack.key}</span>
                      <span>•</span>
                      <span>{currentTrack.genre}</span>
                    </div>
                  </div>
                </div>
              </aside>
            </div>
          </div>
        )}

        {/* 2. NOW PLAYING // VINYL GATEFOLD */}
        {currentView === "now-playing" && (
          <div className="editorial-now-playing-gatefold">
            <div className="gatefold-jacket">
              {/* Left Sleeve: Giant Artwork */}
              <div className="jacket-left-sleeve">
                <div className="sleeve-card">
                  <img src={currentTrack.coverArt} alt={currentTrack.title} className="sleeve-artwork" />
                  <div className="sleeve-caption">
                    <span>RECORDING VENUE: ECHO CHAMBER B, TOKYO</span>
                    <span>MASTER: 1/2-INCH MAGNETIC TAPE</span>
                  </div>
                </div>
              </div>

              {/* Right Sleeve: Liner Notes, Lyrics & Needle Transport */}
              <div className="jacket-right-sleeve">
                <div className="sleeve-kicker">CURRENT GATEFOLD MASTER</div>
                <h1 className="sleeve-title">{currentTrack.title}</h1>
                <p className="sleeve-artist">{currentTrack.artist}</p>

                {/* Poetic Stanzas */}
                <div className="sleeve-lyrics-box">
                  <div className="lyrics-head">SYNCHRONIZED LYRIC STANZAS</div>
                  <p className="lyrics-verse">{currentTrack.lyricsSnippet || "Pure instrumental movement. Bass frequencies reverberating through the hall..."}</p>
                </div>

                {/* Needle Scrubber */}
                <div className="analog-needle-scrubber">
                  <div className="needle-time">
                    <span>{formatTime(playbackSecs)}</span>
                    <span>{formatTime(currentTrack.durationSecs || 240)}</span>
                  </div>
                  <div
                    className="needle-groove-track"
                    onClick={(e) => {
                      const rect = e.currentTarget.getBoundingClientRect();
                      onSeek(Math.floor(((e.clientX - rect.left) / rect.width) * (currentTrack.durationSecs || 240)));
                    }}
                  >
                    <div className="needle-groove-fill" style={{ width: `${(playbackSecs / (currentTrack.durationSecs || 240)) * 100}%` }} />
                  </div>
                </div>

                {/* Transport Buttons */}
                <div className="analog-transport-buttons">
                  <button type="button" onClick={onPrevTrack}><SkipBack size={18} /></button>
                  <button type="button" className="btn-play-analog" onClick={onTogglePlayPause}>
                    {isPlaying ? <Pause size={20} fill="#141312" /> : <Play size={20} fill="#141312" />}
                  </button>
                  <button type="button" onClick={onNextTrack}><SkipForward size={18} /></button>
                  <button type="button" onClick={() => onToggleLike(currentTrack.id)}>
                    <Heart size={18} fill={isLiked ? "#c4683c" : "none"} color={isLiked ? "#c4683c" : "currentColor"} />
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* 3. DISCOVERY // GAZETTE REVIEWS */}
        {currentView === "discovery" && (
          <div className="editorial-discovery-view">
            <div className="gazette-header">
              <h2>The Acoustic Review // Critical Selections</h2>
              <p>Critical survey of worldwide recordings and new pressings.</p>
            </div>

            <div className="gazette-columns-grid">
              {tracks.map((t, idx) => (
                <div key={t.id} className="gazette-review-card" onClick={() => onPlayTrack(t)}>
                  <span className="review-num">N° 0{idx + 1}</span>
                  <img src={t.coverArt} alt={t.title} className="review-art" />
                  <h4 className="review-title">{t.title}</h4>
                  <div className="review-artist">{t.artist}</div>
                  <p className="review-snippet">{t.linerNotes || "An extraordinary achievement in analog dynamics, pairing vintage drum modules with acoustic strings."}</p>
                  <div className="review-footer">
                    <span>{t.genre}</span>
                    <Play size={13} />
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* 4. ARCHIVAL VAULT & ALBUMS */}
        {(currentView === "library" || currentView === "albums") && (
          <div className="editorial-library-view">
            <div className="vault-masthead">
              <div className="vault-title-group">
                <h2>{currentView === "albums" ? "Catalog of Master Pressings" : "Archival Listening Index"}</h2>
                <p>Complete registered holdings: {tracks.length} compositions across {albums.length} boxed pressings.</p>
              </div>
              <div className="vault-search-box">
                <Search size={14} />
                <input
                  type="text"
                  placeholder="Filter archival catalog..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                />
              </div>
            </div>

            <div className="sleeve-tracklist-table">
              <div className="table-head">
                <span>#</span>
                <span>COMPOSITION</span>
                <span>ARTIST</span>
                <span>FORMAT</span>
                <span>TIME</span>
                <span></span>
              </div>
              {tracks.map((t, idx) => (
                <div key={t.id} className="table-row" onClick={() => onPlayTrack(t)}>
                  <span className="row-num">{romanNumerals[idx] || idx + 1}.</span>
                  <span className="row-title"><strong>{t.title}</strong> — {t.album}</span>
                  <span className="row-artist">{t.artist}</span>
                  <span className="row-format">{t.genre}</span>
                  <span className="row-time">{formatTime(t.durationSecs)}</span>
                  <button type="button" className="row-play-btn">
                    <Play size={14} />
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* 5. BOX SETS & PLAYLISTS */}
        {(currentView === "playlists" || currentView === "liked") && (
          <div className="editorial-playlists-view">
            <div className="boxset-masthead">
              <h2>{currentView === "liked" ? "The Private Collection" : "Curator's Box Sets & Anthologies"}</h2>
              <p>Hand-bound audio anthologies and algorithmic smart suites.</p>
            </div>

            <div className="boxset-gallery">
              {playlists.map((pl) => (
                <div key={pl.id} className="boxset-item" onClick={() => onPlayTrack(pl.tracks[0] || tracks[0])}>
                  <img src={pl.coverArt} alt={pl.name} />
                  <div className="boxset-details">
                    <span className="boxset-edition">{pl.isSmartMix ? "LIMITED SMART PRESSING" : "PUBLIC ANTHOLOGY"}</span>
                    <h3>{pl.name}</h3>
                    <p>{pl.description}</p>
                    <span className="boxset-vol">{pl.trackCount} Compositions</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* 6. STATS & INDEX */}
        {currentView === "stats" && (
          <div className="editorial-stats-view">
            <h2>The Listener's Annual Monograph</h2>
            <div className="stats-monograph-essay">
              <p className="essay-drop">
                Over the past calendar year, your acoustic sessions have encompassed <strong>142.8 cumulative hours</strong>. Your listening profile shows an overwhelming affinity for contemplative downtime, acoustic physical modeling, and tape-fluttered city melodies.
              </p>
              <div className="stats-editorial-quote">
                “Music is not an ambient commodity. It is the architectural soundtrack of your days.”
              </div>
            </div>
          </div>
        )}

        {/* 7. SETTINGS & COLOPHON */}
        {currentView === "settings" && (
          <div className="editorial-settings-view">
            <h2>Colophon & Technical Standards</h2>
            <div className="colophon-block">
              <div className="colophon-row">
                <span>Application Runtime</span>
                <strong>Kaze Audio Engine v1.0.7 / Linux Alsa Direct</strong>
              </div>
              <div className="colophon-row">
                <span>Window Management</span>
                <strong>Desktop Tray Mode (Minimize to tray on close)</strong>
              </div>
              <div className="colophon-row">
                <span>Typography</span>
                <strong>Cormorant Garamond (Serif) & Inter (Grotesque)</strong>
              </div>
            </div>
          </div>
        )}
      </main>

      {/* Persistent Archival Audio Transport Bar */}
      <footer className="model-c-transport-bar">
        <div className="transport-track-info">
          <span className="transport-roman">NOW AUDITIONING:</span>
          <strong>{currentTrack.title}</strong>
          <span>by {currentTrack.artist}</span>
        </div>

        <div className="transport-buttons">
          <button type="button" onClick={onPrevTrack}><SkipBack size={15} /></button>
          <button type="button" className="btn-transport-play" onClick={onTogglePlayPause}>
            {isPlaying ? <Pause size={16} fill="#141312" /> : <Play size={16} fill="#141312" />}
          </button>
          <button type="button" onClick={onNextTrack}><SkipForward size={15} /></button>
        </div>

        <div className="transport-timecode">
          <span>{formatTime(playbackSecs)} / {formatTime(currentTrack.durationSecs || 240)}</span>
          <div className="transport-vol-group">
            <Volume2 size={14} />
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={volume}
              onChange={(e) => onVolumeChange(parseFloat(e.target.value))}
              className="transport-vol-slider"
            />
          </div>
        </div>
      </footer>
    </div>
  );
};
