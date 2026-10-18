import React, { useState } from "react";
import {
  Compass,
  Sparkles,
  Music,
  Disc,
  ListMusic,
  Heart,
  Activity,
  Sliders,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Shuffle,
  Repeat,
  Radio,
  Search,
  Volume2,
  Info,
  Flame,
  Folder,
} from "lucide-react";
import { ExplorationAppContext, AppViewId, ExplorationTrack } from "../../types";

export const ModelAApp: React.FC<ExplorationAppContext> = (ctx) => {
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
    isShuffled,
    repeatMode,
    isLiked,
    searchQuery,
    setSearchQuery,
    onPlayTrack,
    onTogglePlayPause,
    onNextTrack,
    onPrevTrack,
    onSeek,
    onVolumeChange,
    onToggleShuffle,
    onToggleRepeat,
    onToggleLike,
  } = ctx;

  const [flowSteering, setFlowSteering] = useState(0.65);
  const [showLinerNotes, setShowLinerNotes] = useState(false);
  const [vectorGeometry, setVectorGeometry] = useState<"arc" | "diagonal">("arc");

  // Derive previous (up to 2) and next (up to 3) tracks for Now Playing
  const currentIdx = tracks.findIndex((t) => t.id === currentTrack.id);
  const prevTracks: ExplorationTrack[] = [];
  if (currentIdx > 0) prevTracks.push(tracks[currentIdx - 1]);
  if (currentIdx > 1) prevTracks.unshift(tracks[currentIdx - 2]);

  const nextTracks: ExplorationTrack[] = [];
  for (let i = 1; i <= 3; i++) {
    if (currentIdx + i < tracks.length) {
      nextTracks.push(tracks[currentIdx + i]);
    }
  }

  const formatTime = (secs: number) => {
    const mins = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${mins}:${s.toString().padStart(2, "0")}`;
  };

  const navItems: Array<{ id: AppViewId; label: string; icon: React.ReactNode }> = [
    { id: "home", label: "Flow & Home", icon: <Compass size={16} /> },
    { id: "now-playing", label: "Orbit Player", icon: <Radio size={16} /> },
    { id: "discovery", label: "Discovery", icon: <Sparkles size={16} /> },
    { id: "library", label: "Library", icon: <Music size={16} /> },
    { id: "albums", label: "Albums", icon: <Disc size={16} /> },
    { id: "playlists", label: "Playlists", icon: <ListMusic size={16} /> },
    { id: "liked", label: "Liked Songs", icon: <Heart size={16} /> },
    { id: "stats", label: "Sonic Stats", icon: <Activity size={16} /> },
    { id: "settings", label: "Settings", icon: <Sliders size={16} /> },
  ];

  const progressPercent = Math.min(100, (playbackSecs / (currentTrack.durationSecs || 240)) * 100);

  return (
    <div className="model-a-fullapp">
      {/* Dynamic ambient ember aura from track accent */}
      <div
        className="model-a-ambient-glow"
        style={{
          background: `radial-gradient(ellipse 60% 45% at 50% 35%, ${currentTrack.accentColor || "#f3701e"}20 0%, transparent 70%)`,
        }}
      />

      {/* Left Kinetic Floating Dock */}
      <aside className="model-a-sidebar" aria-label="Kinetic Navigation Dock">
        <div className="model-a-brand" onClick={() => setCurrentView("home")}>
          <div className="brand-orb" style={{ backgroundColor: currentTrack.accentColor || "#f3701e" }} />
          <span className="brand-title">KAZE</span>
          <span className="brand-mode">ORBIT</span>
        </div>

        <nav className="model-a-nav-list">
          {navItems.map((item) => {
            const isActive = currentView === item.id;
            return (
              <button
                key={item.id}
                type="button"
                className={`model-a-nav-btn ${isActive ? "active" : ""}`}
                onClick={() => setCurrentView(item.id)}
              >
                <span className="nav-icon">{item.icon}</span>
                <span className="nav-label">{item.label}</span>
                {isActive && <span className="nav-active-pill" />}
              </button>
            );
          })}
        </nav>

        {/* Mini Now-Playing Orb Trigger */}
        <div className="model-a-sidebar-player-pill" onClick={() => setCurrentView("now-playing")}>
          <div className="pill-thumb-wrap">
            <img src={currentTrack.coverArt} alt={currentTrack.title} className="pill-thumb" />
            {isPlaying && <div className="pill-pulse" />}
          </div>
          <div className="pill-text">
            <div className="pill-title">{currentTrack.title}</div>
            <div className="pill-artist">{currentTrack.artist}</div>
          </div>
        </div>
      </aside>

      {/* Main App Stage Area */}
      <div className="model-a-main-stage">
        {/* Top Header Bar */}
        <header className="model-a-header">
          <div className="header-breadcrumbs">
            <span className="view-kicker">MODEL A // KINETIC FLOW</span>
            <span className="view-divider">/</span>
            <span className="active-view-name">{currentView.toUpperCase()}</span>
          </div>

          <div className="header-search-wrap">
            <Search size={14} className="search-icon" />
            <input
              type="text"
              placeholder="Search library, artists, or soundscapes..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="model-a-search-input"
            />
          </div>

          <div className="header-telemetry-pill">
            <span className="telemetry-dot" />
            <span>FLOW ENGINE // ACTIVE</span>
          </div>
        </header>

        {/* View Router */}
        <div className="model-a-content-scrollable">
          {/* 1. NOW PLAYING / ORBIT VIEW */}
          {currentView === "now-playing" && (
            <div className="model-a-now-playing-view">
              {/* Geometry Submode Switcher */}
              <div className="orbit-submode-bar">
                <span className="submode-label">VECTOR GEOMETRY:</span>
                <div className="submode-toggles">
                  <button
                    type="button"
                    className={`submode-btn ${vectorGeometry === "arc" ? "active" : ""}`}
                    onClick={() => setVectorGeometry("arc")}
                  >
                    <span>∿ PARABOLIC DRIFT ARC</span>
                  </button>
                  <button
                    type="button"
                    className={`submode-btn ${vectorGeometry === "diagonal" ? "active" : ""}`}
                    onClick={() => setVectorGeometry("diagonal")}
                  >
                    <span>⤢ CRISP DIAGONAL VECTOR</span>
                  </button>
                </div>
              </div>

              <div className={`orbit-hero-container ${vectorGeometry === "diagonal" ? "mode-diagonal" : "mode-arc"}`}>
                {/* Hairline guide rays for crisp diagonal mode */}
                {vectorGeometry === "diagonal" && (
                  <div className="vector-diagonal-guide-overlay" aria-hidden="true">
                    <div className="guide-ray left-ray">
                      <span className="ray-coord">VECTOR ↙ -35.0°</span>
                    </div>
                    <div className="guide-ray right-ray">
                      <span className="ray-coord">VECTOR ↘ +35.0°</span>
                    </div>
                  </div>
                )}

                {/* Left Drift: Previous Tracks */}
                <div className="orbit-drift-wing left-wing">
                  <div className="wing-label">
                    {vectorGeometry === "diagonal" ? "VECTOR ↙ -35.0° TRAILING" : "PREVIOUS DRIFT"}
                  </div>
                  {prevTracks.length === 0 ? (
                    <div className="wing-empty">Queue Head</div>
                  ) : (
                    prevTracks.map((t, idx) => (
                      <div
                        key={t.id}
                        className={`drift-card prev-card tier-${idx} ${vectorGeometry === "diagonal" ? "crisp-vec-card" : ""}`}
                        onClick={() => onPlayTrack(t)}
                      >
                        <img src={t.coverArt} alt={t.title} className="drift-card-img" />
                        <div className="drift-card-meta">
                          <div className="drift-title">{t.title}</div>
                          <div className="drift-artist">{t.artist}</div>
                          {vectorGeometry === "diagonal" && (
                            <div className="drift-vector-badge">VEC ↙ -35° • TIER {idx + 1}</div>
                          )}
                        </div>
                      </div>
                    ))
                  )}
                </div>

                {/* Center: Massive Monolithic Hero Playing Card */}
                <div className="orbit-hero-centerpiece">
                  <div className="hero-artwork-housing">
                    <div className={`hero-vinyl-backing ${isPlaying ? "spinning" : ""}`} />
                    <img src={currentTrack.coverArt} alt={currentTrack.title} className="hero-main-cover" />
                  </div>

                  <div className="hero-metadata">
                    <span className="hero-genre-tag" style={{ color: currentTrack.accentColor }}>
                      {currentTrack.genre.toUpperCase()}
                    </span>
                    <h1 className="hero-track-title">{currentTrack.title}</h1>
                    <p className="hero-track-artist">{currentTrack.artist} — {currentTrack.album}</p>
                  </div>

                  {/* Scrubber */}
                  <div className="hero-scrubber">
                    <div className="time-row">
                      <span>{formatTime(playbackSecs)}</span>
                      <span>{formatTime(currentTrack.durationSecs || 240)}</span>
                    </div>
                    <div
                      className="scrubber-bar-track"
                      onClick={(e) => {
                        const rect = e.currentTarget.getBoundingClientRect();
                        const ratio = (e.clientX - rect.left) / rect.width;
                        onSeek(Math.floor(ratio * (currentTrack.durationSecs || 240)));
                      }}
                    >
                      <div className="scrubber-bar-fill" style={{ width: `${progressPercent}%`, backgroundColor: currentTrack.accentColor }} />
                    </div>
                  </div>

                  {/* Transport */}
                  <div className="hero-transport-deck">
                    <button type="button" className={`transport-btn-sub ${isShuffled ? "active" : ""}`} onClick={onToggleShuffle}>
                      <Shuffle size={16} />
                    </button>
                    <button type="button" className="transport-btn-nav" onClick={onPrevTrack}>
                      <SkipBack size={20} />
                    </button>
                    <button
                      type="button"
                      className="transport-btn-hero-play"
                      style={{ backgroundColor: currentTrack.accentColor || "#f3701e" }}
                      onClick={onTogglePlayPause}
                    >
                      {isPlaying ? <Pause size={24} fill="#ffffff" /> : <Play size={24} fill="#ffffff" />}
                    </button>
                    <button type="button" className="transport-btn-nav" onClick={onNextTrack}>
                      <SkipForward size={20} />
                    </button>
                    <button type="button" className={`transport-btn-sub ${repeatMode !== "off" ? "active" : ""}`} onClick={onToggleRepeat}>
                      <Repeat size={16} />
                    </button>
                    <button type="button" className={`transport-btn-sub ${isLiked ? "liked" : ""}`} onClick={() => onToggleLike(currentTrack.id)}>
                      <Heart size={16} fill={isLiked ? "#f3701e" : "none"} color={isLiked ? "#f3701e" : "currentColor"} />
                    </button>
                  </div>
                </div>

                {/* Right Drift: Upcoming Queue */}
                <div className="orbit-drift-wing right-wing">
                  <div className="wing-label">
                    {vectorGeometry === "diagonal" ? "VECTOR ↘ +35.0° FORWARD" : "NEXT TRAJECTORY"}
                  </div>
                  {nextTracks.length === 0 ? (
                    <div className="wing-empty">Queue End</div>
                  ) : (
                    nextTracks.map((t, idx) => (
                      <div
                        key={t.id}
                        className={`drift-card next-card tier-${idx + 1} ${vectorGeometry === "diagonal" ? "crisp-vec-card" : ""}`}
                        onClick={() => onPlayTrack(t)}
                      >
                        <img src={t.coverArt} alt={t.title} className="drift-card-img" />
                        <div className="drift-card-meta">
                          <div className="drift-title">{t.title}</div>
                          <div className="drift-artist">{t.artist}</div>
                          {vectorGeometry === "diagonal" ? (
                            <div className="drift-vector-badge">VEC ↘ +35° • +{idx + 1} AHEAD</div>
                          ) : (
                            <div className="drift-index">+{idx + 1} NEXT</div>
                          )}
                        </div>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </div>
          )}

          {/* 2. HOME / FLOW VIEW */}
          {currentView === "home" && (
            <div className="model-a-home-view">
              {/* Massive Flow Chamber Banner */}
              <section className="model-a-flow-chamber">
                <div className="flow-chamber-header">
                  <div className="chamber-badge">
                    <Flame size={14} />
                    <span>ALGORITHMIC FLOW ENGINE</span>
                  </div>
                  <h2>Continuum of Sound</h2>
                  <p>Adaptive listening that evolves in real-time based on your acoustic resonance.</p>
                </div>

                <div className="flow-steering-panel">
                  <div className="steering-labels">
                    <span>FAMILIAR RESONANCE</span>
                    <span className="current-steering-val">{(flowSteering * 100).toFixed(0)}% EXPLORATION</span>
                    <span>WILD DISCOVERY</span>
                  </div>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={flowSteering}
                    onChange={(e) => setFlowSteering(parseFloat(e.target.value))}
                    className="model-a-range-slider"
                  />
                  <div className="chamber-actions">
                    <button type="button" className="btn-start-flow" onClick={onTogglePlayPause}>
                      {isPlaying ? "PAUSE FLOW SESSION" : "START FLOW SESSION"}
                    </button>
                    <button type="button" className="btn-why-track" onClick={() => setShowLinerNotes(!showLinerNotes)}>
                      <Info size={14} />
                      <span>WHY THIS TRACK?</span>
                    </button>
                  </div>
                </div>

                {showLinerNotes && (
                  <div className="flow-explanation-card">
                    <strong>Acoustic Rationale:</strong> Selected because you frequently enjoy high-cadence electronic rhythms paired with organic acoustic instruments around this hour.
                  </div>
                )}
              </section>

              {/* Asymmetric Staggered Wave: Recent Rotation */}
              <section className="model-a-section">
                <div className="section-title-row">
                  <h3 className="section-title">Recent Gravitational Orbit</h3>
                  <span className="section-subtitle">Tracks in heavy rotation</span>
                </div>
                <div className="staggered-track-grid">
                  {tracks.slice(0, 6).map((track, i) => (
                    <div
                      key={track.id}
                      className={`staggered-track-card card-slant-${(i % 3) + 1}`}
                      onClick={() => onPlayTrack(track)}
                    >
                      <div className="staggered-cover-wrap">
                        <img src={track.coverArt} alt={track.title} />
                        <div className="card-play-hover">
                          <Play size={18} fill="#ffffff" />
                        </div>
                      </div>
                      <div className="staggered-meta">
                        <div className="staggered-title">{track.title}</div>
                        <div className="staggered-artist">{track.artist}</div>
                        <span className="staggered-tag">{track.genre}</span>
                      </div>
                    </div>
                  ))}
                </div>
              </section>
            </div>
          )}

          {/* 3. DISCOVERY VIEW */}
          {currentView === "discovery" && (
            <div className="model-a-discovery-view">
              <div className="discovery-header-banner">
                <span className="discovery-pill">WORLDWIDE SONIC RADAR</span>
                <h1>Trending Frequency & Top Charts</h1>
                <p>Real-time universal popularity feeds across global storefronts.</p>
              </div>

              {/* Velocity Trends */}
              <div className="velocity-stream">
                {tracks.map((track, i) => (
                  <div
                    key={track.id}
                    className="velocity-row"
                    onClick={() => onPlayTrack(track)}
                  >
                    <span className="rank-num">0{i + 1}</span>
                    <img src={track.coverArt} alt={track.title} className="velocity-thumb" />
                    <div className="velocity-info">
                      <div className="velocity-title">{track.title}</div>
                      <div className="velocity-artist">{track.artist} • {track.album}</div>
                    </div>
                    <span className="velocity-genre">{track.genre}</span>
                    <span className="velocity-bpm">{track.bpm} BPM</span>
                    <button type="button" className="velocity-play-btn">
                      <Play size={14} fill="currentColor" />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* 4. LIBRARY & ALBUMS VIEW */}
          {(currentView === "library" || currentView === "albums") && (
            <div className="model-a-library-view">
              <div className="library-header-row">
                <h2>{currentView === "albums" ? "Album Vault & Pressings" : "Complete Master Library"}</h2>
                <div className="library-stats-pill">{tracks.length} MASTER TRACKS • {albums.length} ALBUMS</div>
              </div>

              {currentView === "albums" ? (
                <div className="albums-vinyl-shelf">
                  {albums.map((alb) => (
                    <div key={alb.id} className="vinyl-album-card" onClick={() => onPlayTrack(alb.tracks[0] || tracks[0])}>
                      <div className="album-sleeve-frame">
                        <img src={alb.coverArt} alt={alb.title} />
                        <div className="vinyl-groove-peek" />
                      </div>
                      <div className="album-card-title">{alb.title}</div>
                      <div className="album-card-artist">{alb.artist} ({alb.year})</div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="constellation-track-list">
                  {tracks.map((t, idx) => (
                    <div key={t.id} className="constellation-row" onClick={() => onPlayTrack(t)}>
                      <span className="row-index">{idx + 1}</span>
                      <img src={t.coverArt} alt={t.title} className="row-art" />
                      <div className="row-name">
                        <div className="row-title">{t.title}</div>
                        <div className="row-sub">{t.artist} — {t.album}</div>
                      </div>
                      <span className="row-key">{t.key}</span>
                      <span className="row-time">{formatTime(t.durationSecs)}</span>
                      <button type="button" className="row-like-btn" onClick={(e) => { e.stopPropagation(); onToggleLike(t.id); }}>
                        <Heart size={14} fill={t.isLiked ? "#f3701e" : "none"} />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* 5. PLAYLISTS & LIKED VIEW */}
          {(currentView === "playlists" || currentView === "liked") && (
            <div className="model-a-playlists-view">
              <div className="playlists-header">
                <h2>{currentView === "liked" ? "Liked Songs & Favorites" : "Curated Playlists & Smart Mixes"}</h2>
                <p>Archival collections organized by acoustic intent and listener curation.</p>
              </div>

              <div className="playlists-asymmetric-grid">
                {playlists.map((pl) => (
                  <div key={pl.id} className="playlist-box-card" onClick={() => onPlayTrack(pl.tracks[0] || tracks[0])}>
                    <img src={pl.coverArt} alt={pl.name} className="box-cover" />
                    <div className="box-content">
                      <span className="box-badge">{pl.isSmartMix ? "ALGORITHMIC MIX" : "PLAYLIST"}</span>
                      <h4 className="box-name">{pl.name}</h4>
                      <p className="box-desc">{pl.description}</p>
                      <div className="box-footer">
                        <span>{pl.trackCount} Tracks</span>
                        <Play size={14} />
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* 6. STATS VIEW */}
          {currentView === "stats" && (
            <div className="model-a-stats-view">
              <div className="stats-header">
                <h2>Sonic Footprint & Acoustic Radar</h2>
                <p>Telemetry recorded across your listening lifecycle.</p>
              </div>

              <div className="stats-cards-grid">
                <div className="stats-metric-card">
                  <div className="metric-label">TOTAL HOURS LISTENED</div>
                  <div className="metric-huge">142.8</div>
                  <div className="metric-sub">Across 864 individual listening sessions</div>
                </div>

                <div className="stats-metric-card">
                  <div className="metric-label">DOMINANT HARMONIC GENRE</div>
                  <div className="metric-huge accent">Downtempo</div>
                  <div className="metric-sub">38% of your overall acoustic energy</div>
                </div>

                <div className="stats-metric-card wide-card">
                  <div className="metric-label">TOP ARTISTS BY AFFINITY</div>
                  <div className="affinity-list">
                    {["Kurogane Sound Syndicate (48h)", "Aethelgard & Sylvan (34h)", "Tetsuo Kanno (28h)"].map((art, idx) => (
                      <div key={idx} className="affinity-row">
                        <span>0{idx + 1}. {art}</span>
                        <div className="affinity-bar-bg">
                          <div className="affinity-bar-fill" style={{ width: `${95 - idx * 15}%` }} />
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* 7. SETTINGS VIEW */}
          {currentView === "settings" && (
            <div className="model-a-settings-view">
              <h2>System Architecture & Configuration</h2>
              <div className="settings-panel-box">
                <div className="setting-row">
                  <div className="setting-info">
                    <span className="setting-title">Close Button Behavior</span>
                    <span className="setting-desc">Keep Kaze active in system tray when window is closed</span>
                  </div>
                  <span className="setting-value-badge">MINIMIZE TO TRAY</span>
                </div>

                <div className="setting-row">
                  <div className="setting-info">
                    <span className="setting-title">Audio Output Engine</span>
                    <span className="setting-desc">Direct backend Rodio / Symphonia ALSA stream</span>
                  </div>
                  <span className="setting-value-badge">ALSA BIT-PERFECT</span>
                </div>

                <div className="setting-row">
                  <div className="setting-info">
                    <span className="setting-title">Configured Music Folders</span>
                    <span className="setting-desc">Local directories scanned for lossless audio</span>
                  </div>
                  <button type="button" className="btn-rescan">
                    <Folder size={14} />
                    <span>MANAGE DIRECTORIES</span>
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Persistent Bottom Kinetic Bar (when outside Now Playing) */}
        {currentView !== "now-playing" && (
          <footer className="model-a-bottom-bar" onClick={() => setCurrentView("now-playing")}>
            <div className="bar-track-summary">
              <img src={currentTrack.coverArt} alt={currentTrack.title} className="bar-art" />
              <div className="bar-meta">
                <div className="bar-title">{currentTrack.title}</div>
                <div className="bar-artist">{currentTrack.artist}</div>
              </div>
            </div>

            <div className="bar-controls" onClick={(e) => e.stopPropagation()}>
              <button type="button" className="bar-nav-btn" onClick={onPrevTrack}>
                <SkipBack size={18} />
              </button>
              <button type="button" className="bar-play-btn" onClick={onTogglePlayPause}>
                {isPlaying ? <Pause size={18} fill="#ffffff" /> : <Play size={18} fill="#ffffff" />}
              </button>
              <button type="button" className="bar-nav-btn" onClick={onNextTrack}>
                <SkipForward size={18} />
              </button>
            </div>

            <div className="bar-time">
              <span>{formatTime(playbackSecs)} / {formatTime(currentTrack.durationSecs || 240)}</span>
              <div className="bar-vol-ctrl" onClick={(e) => e.stopPropagation()}>
                <Volume2 size={15} />
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={volume}
                  onChange={(e) => onVolumeChange(parseFloat(e.target.value))}
                  className="bar-vol-slider"
                />
              </div>
            </div>
          </footer>
        )}
      </div>
    </div>
  );
};
