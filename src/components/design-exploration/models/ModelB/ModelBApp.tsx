import React, { useState, useRef } from "react";
import {
  Layers,
  Compass,
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
  Volume2,
  ChevronUp,
  ChevronDown,
  Search,
  ArrowRight,
  Flame,
} from "lucide-react";
import { ExplorationAppContext, AppViewId } from "../../types";
import { CATEGORY_STAGES } from "../../mockData";
import { DraggableRow } from "../../DraggableRow";

export const ModelBApp: React.FC<ExplorationAppContext> = (ctx) => {
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

  const [activeStageIndex, setActiveStageIndex] = useState(0);
  const [isTransitioning, setIsTransitioning] = useState(false);
  const wheelTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const stages = CATEGORY_STAGES;
  const currentStage = stages[activeStageIndex] || stages[0];

  const changeStage = (newIndex: number) => {
    if (newIndex < 0 || newIndex >= stages.length || newIndex === activeStageIndex) return;
    setIsTransitioning(true);
    setActiveStageIndex(newIndex);
    setTimeout(() => setIsTransitioning(false), 260);
  };

  const handleStageWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    if (Math.abs(e.deltaX) > Math.abs(e.deltaY)) return; // let horizontal trackpad scroll cards
    if (Math.abs(e.deltaY) < 25) return;
    if (wheelTimeoutRef.current) return;

    if (e.deltaY > 0) {
      if (activeStageIndex < stages.length - 1) changeStage(activeStageIndex + 1);
    } else {
      if (activeStageIndex > 0) changeStage(activeStageIndex - 1);
    }

    wheelTimeoutRef.current = setTimeout(() => {
      wheelTimeoutRef.current = null;
    }, 450);
  };

  const formatTime = (secs: number) => {
    const mins = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${mins}:${s.toString().padStart(2, "0")}`;
  };

  const navTabs: Array<{ id: AppViewId; label: string; code: string; icon: React.ReactNode }> = [
    { id: "now-playing", label: "CONSTRUCTIVIST PLAYER", code: "001", icon: <Layers size={14} /> },
    { id: "discovery", label: "STAGE DISCOVERY", code: "002", icon: <Layers size={14} /> },
    { id: "home", label: "SIGNAL DISPATCH", code: "003", icon: <Compass size={14} /> },
    { id: "library", label: "ARCHIVE LEDGER", code: "004", icon: <Music size={14} /> },
    { id: "albums", label: "MASTER PRESSINGS", code: "005", icon: <Disc size={14} /> },
    { id: "playlists", label: "MATRIX SUITES", code: "006", icon: <ListMusic size={14} /> },
    { id: "stats", label: "MACHINE AUDIT", code: "007", icon: <Activity size={14} /> },
    { id: "settings", label: "CALIBRATION", code: "008", icon: <Sliders size={14} /> },
  ];

  return (
    <div className="model-b-fullapp brutalist-theme">
      {/* Top Brutalist Coordinate Ruler */}
      <header className="model-b-header brutalist-masthead">
        <div className="brutalist-system-telemetry">
          <span className="telemetry-coord">SYSTEM: 24-BIT / 96kHz ALSA DIRECT</span>
          <span className="telemetry-coord">STATUS: BIT-PERFECT</span>
          <span className="telemetry-coord">BUFFER: 64 SAMPLES // 0.67ms</span>
        </div>

        <div className="brutalist-masthead-row">
          <div className="model-b-brand brutalist-brand" onClick={() => setCurrentView("now-playing")}>
            <span className="brand-code">02 // BRUTALISM</span>
            <span className="brand-name">KAZE STRUCT</span>
          </div>

          {/* Quick Search */}
          <div className="brutalist-search-cell">
            <Search size={14} />
            <input
              type="text"
              placeholder="QUERY TECHNICAL MATRIX..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
        </div>

        {/* Center Nav Ruler Strip */}
        <nav className="brutalist-nav-strip">
          {navTabs.map((tab) => {
            const isActive = currentView === tab.id;
            return (
              <button
                key={tab.id}
                type="button"
                className={`brutalist-ruler-btn ${isActive ? "active" : ""}`}
                onClick={() => setCurrentView(tab.id)}
              >
                <span className="ruler-code">{tab.code}</span>
                <span className="ruler-label">{tab.label}</span>
                {isActive && <span className="ruler-active-bar" />}
              </button>
            );
          })}
        </nav>
      </header>

      {/* Main App Stage Container */}
      <div className="model-b-body">
        {/* 0. NOW PLAYING // CONSTRUCTIVIST PLAYER */}
        {currentView === "now-playing" && (
          <div className="brutalist-now-playing-view">
            <div className="brutalist-split-player">
              {/* Left Bay: Raw Artwork with Crosshairs */}
              <div className="brutalist-art-bay">
                <div className="crosshair-frame">
                  <span className="ch-mark top-left">+</span>
                  <span className="ch-mark top-right">+</span>
                  <span className="ch-mark bottom-left">+</span>
                  <span className="ch-mark bottom-right">+</span>
                  <img src={currentTrack.coverArt} alt={currentTrack.title} className="brutalist-raw-art" />
                </div>
                <div className="art-manifest-bar">
                  <span>ID: #KZ-9921-RAW</span>
                  <span>{currentTrack.genre.toUpperCase()}</span>
                  <span>{currentTrack.bpm} BPM</span>
                </div>
              </div>

              {/* Right Bay: Massive Typography & Mechanical Transport */}
              <div className="brutalist-spec-bay">
                <div className="spec-serial-kicker">01 // ACTIVE MASTER RECORDING</div>
                <h1 className="brutalist-hero-title">{currentTrack.title}</h1>
                <div className="brutalist-artist-line">
                  <strong>{currentTrack.artist}</strong>
                  <span>//</span>
                  <span>{currentTrack.album}</span>
                </div>

                <div className="brutalist-metrics-grid">
                  <div className="metric-box">
                    <span className="metric-lbl">ENCODING</span>
                    <strong className="metric-val">24-BIT / 96kHz</strong>
                  </div>
                  <div className="metric-box">
                    <span className="metric-lbl">KEY HARMONIC</span>
                    <strong className="metric-val">{currentTrack.key}</strong>
                  </div>
                  <div className="metric-box">
                    <span className="metric-lbl">PLAYBACK STATUS</span>
                    <strong className="metric-val">{isPlaying ? "ALSA STREAMING" : "STREAM PAUSED"}</strong>
                  </div>
                  <div className="metric-box">
                    <span className="metric-lbl">OUTPUT LEVEL</span>
                    <strong className="metric-val">-14.2 LUFS</strong>
                  </div>
                </div>

                {/* Quantized Stepped Spectrum Meter */}
                <div className="brutalist-stepped-spectrum">
                  {Array.from({ length: 24 }).map((_, i) => {
                    const lit = isPlaying ? (i * 7 + Math.floor(playbackSecs * 4)) % 10 > 2 : i < 3;
                    return (
                      <div key={i} className={`step-bar ${lit ? "lit" : ""}`} />
                    );
                  })}
                </div>

                {/* Mechanical Transport Bar */}
                <div className="brutalist-transport-cluster">
                  <button type="button" className="b-trans-btn" onClick={onPrevTrack}>
                    <span>[◀ REW]</span>
                  </button>
                  <button type="button" className={`b-trans-btn play-btn ${isPlaying ? "active-play" : ""}`} onClick={onTogglePlayPause}>
                    <span>{isPlaying ? "[❚❚ PAUSE]" : "[▶ PLAY]"}</span>
                  </button>
                  <button type="button" className="b-trans-btn" onClick={onNextTrack}>
                    <span>[▶ FWD]</span>
                  </button>
                  <button type="button" className={`b-trans-btn ${isLiked ? "liked" : ""}`} onClick={() => onToggleLike(currentTrack.id)}>
                    <span>{isLiked ? "[★ FAV]" : "[☆ FAV]"}</span>
                  </button>
                </div>
              </div>
            </div>

            {/* Bottom Technical Queue Manifest */}
            <div className="brutalist-queue-manifest">
              <div className="manifest-head">
                <span>INDEX</span>
                <span>COMPOSITION</span>
                <span>ARTIST</span>
                <span>TEMPO</span>
                <span>FORMAT</span>
                <span>ACTION</span>
              </div>
              {tracks.slice(0, 6).map((t, idx) => (
                <div
                  key={t.id}
                  className={`manifest-row ${t.id === currentTrack.id ? "active-row" : ""}`}
                  onClick={() => onPlayTrack(t)}
                >
                  <span className="m-idx">0{idx + 1}</span>
                  <span className="m-title">{t.title}</span>
                  <span className="m-artist">{t.artist}</span>
                  <span className="m-tempo">{t.bpm} BPM</span>
                  <span className="m-fmt">FLAC 24/96</span>
                  <button type="button" className="m-btn">AUDITION</button>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* 1. SCROLL STAGE DISCOVERY VIEW */}
        {currentView === "discovery" && (
          <div className="model-b-scroll-stage-view" onWheel={handleStageWheel}>
            <div className="stage-content-zone">
              {/* Top Category Stepper Header */}
              <div className="stage-category-stepper">
                <div className="stepper-kicker">STAGE {currentStage.number} // 0{stages.length}</div>
                <div className="stepper-buttons">
                  <button
                    type="button"
                    className="stepper-btn"
                    disabled={activeStageIndex === 0}
                    onClick={() => changeStage(activeStageIndex - 1)}
                    title="Scroll Up / Previous Stage"
                  >
                    <ChevronUp size={16} />
                  </button>
                  <button
                    type="button"
                    className="stepper-btn"
                    disabled={activeStageIndex === stages.length - 1}
                    onClick={() => changeStage(activeStageIndex + 1)}
                    title="Scroll Down / Next Stage"
                  >
                    <ChevronDown size={16} />
                  </button>
                </div>
              </div>

              {/* Dynamic In-Place Heading Block */}
              <div className={`stage-heading-block ${isTransitioning ? "transitioning" : ""}`}>
                <span className="stage-tag">{currentStage.tag}</span>
                <h1 className="stage-title">{currentStage.title}</h1>
                <p className="stage-subtitle">{currentStage.subtitle}</p>
                <p className="stage-desc">{currentStage.description}</p>
              </div>

              {/* Horizontal Content Band with Mouse-Drag + Touchpad */}
              <div className={`stage-band-wrap ${isTransitioning ? "entering" : ""}`}>
                <div className="drag-hint">
                  <span>Swipe touchpad or click & drag band horizontally</span>
                  <ArrowRight size={12} />
                </div>

                <DraggableRow className="stage-carousel-row">
                  {/* Spotlight Feature Card */}
                  {currentStage.tracks[0] && (
                    <div
                      className="stage-card feature-card"
                      onClick={() => onPlayTrack(currentStage.tracks[0])}
                    >
                      <img src={currentStage.tracks[0].coverArt} alt={currentStage.tracks[0].title} className="card-bg-art" />
                      <div className="card-scrim" />
                      <div className="card-feature-body">
                        <div className="spotlight-tag">
                          <Flame size={12} />
                          <span>SPOTLIGHT MASTER</span>
                        </div>
                        <h3>{currentStage.tracks[0].title}</h3>
                        <p>{currentStage.tracks[0].artist}</p>
                        <div className="card-meta-line">
                          <span>{currentStage.tracks[0].genre}</span>
                          <span>•</span>
                          <span>{currentStage.tracks[0].bpm} BPM</span>
                          <button type="button" className="btn-circle-play">
                            <Play size={15} fill="#0f172a" />
                          </button>
                        </div>
                      </div>
                    </div>
                  )}

                  {/* Standard Track Cards */}
                  {currentStage.tracks.slice(1).map((t) => (
                    <div
                      key={t.id}
                      className="stage-card track-card"
                      onClick={() => onPlayTrack(t)}
                    >
                      <div className="track-art-box">
                        <img src={t.coverArt} alt={t.title} />
                        <div className="play-overlay-pill">
                          <Play size={14} fill="#0f172a" />
                          <span>PLAY</span>
                        </div>
                      </div>
                      <div className="track-card-info">
                        <div className="track-card-title">{t.title}</div>
                        <div className="track-card-artist">{t.artist}</div>
                        <div className="track-card-footer">
                          <span>{t.key}</span>
                          <span>{formatTime(t.durationSecs)}</span>
                        </div>
                      </div>
                    </div>
                  ))}

                  {/* Next Stage Portal Card */}
                  <div
                    className="stage-card next-portal-card"
                    onClick={() => changeStage((activeStageIndex + 1) % stages.length)}
                  >
                    <ArrowRight size={24} />
                    <span>ADVANCE TO</span>
                    <strong>{stages[(activeStageIndex + 1) % stages.length]?.title}</strong>
                  </div>
                </DraggableRow>
              </div>
            </div>

            {/* Right Stage Milestone Rail */}
            <aside className="stage-milestone-rail" aria-label="Category timeline">
              <div className="rail-title">HORIZON MILESTONES</div>
              {stages.map((st, i) => (
                <button
                  key={st.id}
                  type="button"
                  className={`rail-mark ${i === activeStageIndex ? "active" : ""}`}
                  onClick={() => changeStage(i)}
                >
                  <span className="mark-dot" />
                  <div className="mark-text">
                    <span className="mark-num">{st.number}</span>
                    <span className="mark-name">{st.title}</span>
                  </div>
                </button>
              ))}
            </aside>
          </div>
        )}

        {/* 2. SIGNAL DISPATCH // HOME */}
        {currentView === "home" && (
          <div className="brutalist-home-view">
            <div className="dispatch-split-grid">
              {/* Left Column: Autonomous Signal Dispatcher */}
              <div className="dispatch-hero-card">
                <div className="dispatch-kicker">003 // AUTONOMOUS SIGNAL GENERATOR</div>
                <h2 className="dispatch-title">SIGNAL DISPATCH & FLOW CONTINUITY</h2>
                <p className="dispatch-desc">
                  Direct hardware-steered algorithmic dispatch. Dynamically synthesizes queue continuity based on local FLAC acoustic footprint and divergence telemetry.
                </p>

                <div className="dispatch-telemetry-box">
                  <div className="telemetry-cell">
                    <span className="t-label">CURRENT STREAM</span>
                    <strong className="t-val">{currentTrack.title}</strong>
                  </div>
                  <div className="telemetry-cell">
                    <span className="t-label">HARMONIC KEY</span>
                    <strong className="t-val">{currentTrack.key || "C MIN"}</strong>
                  </div>
                  <div className="telemetry-cell">
                    <span className="t-label">TEMPO DRIFT</span>
                    <strong className="t-val">{currentTrack.bpm || 120} BPM</strong>
                  </div>
                  <div className="telemetry-cell">
                    <span className="t-label">ALSA BUFFER</span>
                    <strong className="t-val">64 SAMPLES // 0.67ms</strong>
                  </div>
                </div>

                <div className="dispatch-actions">
                  <button type="button" className="btn-brutalist-dispatch" onClick={onTogglePlayPause}>
                    <span>{isPlaying ? "[❚❚ PAUSE ACTIVE SIGNAL]" : "[▶ INITIATE SIGNAL DISPATCH]"}</span>
                  </button>
                </div>
              </div>

              {/* Right Column: Active Resonance Grid */}
              <div className="dispatch-matrix-bay">
                <div className="matrix-bay-header">
                  <span>ACTIVE TRANSMISSION MATRIX</span>
                  <span>{tracks.length} CANDIDATE NODES</span>
                </div>
                <div className="dispatch-track-grid">
                  {tracks.slice(0, 6).map((t, idx) => (
                    <div key={t.id} className="brutalist-grid-card" onClick={() => onPlayTrack(t)}>
                      <div className="card-top-rule">
                        <span className="card-index">NODE 0{idx + 1}</span>
                        <span className="card-key">{t.key || "C MIN"}</span>
                      </div>
                      <div className="card-thumb-housing">
                        <img src={t.coverArt} alt={t.title} className="card-raw-art" />
                        <div className="card-hover-flag">[AUDITION]</div>
                      </div>
                      <div className="card-meta">
                        <div className="card-title">{t.title}</div>
                        <div className="card-artist">{t.artist}</div>
                        <div className="card-format">24-BIT / {t.bpm} BPM</div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        )}

        {/* 3. ARCHIVE LEDGER // LIBRARY & ALBUMS */}
        {(currentView === "library" || currentView === "albums") && (
          <div className="brutalist-library-view">
            <div className="ledger-masthead">
              <div className="ledger-title-group">
                <h2>ARCHIVE LEDGER // {currentView === "albums" ? "MASTER PRESSINGS" : "LOSSLESS RECORDINGS"}</h2>
                <p>COMPLETE REGISTERED SOUND ASSETS: {tracks.length} TRACKS // {albums.length} ALBUM MASTERS</p>
              </div>
              <div className="ledger-stats-pill">
                <span>BIT-PERFECT DIRECT</span>
              </div>
            </div>

            <div className="ledger-table-chassis">
              <div className="ledger-head">
                <span>SERIAL</span>
                <span>COMPOSITION</span>
                <span>ARTIST</span>
                <span>ALBUM</span>
                <span>TEMPO</span>
                <span>KEY</span>
                <span>FORMAT</span>
                <span>ACTION</span>
              </div>
              {tracks.map((t, idx) => (
                <div
                  key={t.id}
                  className={`ledger-row ${t.id === currentTrack.id ? "active-row" : ""}`}
                  onClick={() => onPlayTrack(t)}
                >
                  <span className="l-serial">#KZ-{1000 + idx}</span>
                  <span className="l-title"><strong>{t.title}</strong></span>
                  <span className="l-artist">{t.artist}</span>
                  <span className="l-album">{t.album}</span>
                  <span className="l-tempo">{t.bpm} BPM</span>
                  <span className="l-key">{t.key}</span>
                  <span className="l-format">FLAC 24/96</span>
                  <button type="button" className="l-action-btn">LOAD</button>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* 4. MATRIX SUITES // PLAYLISTS & FAVORITES */}
        {(currentView === "playlists" || currentView === "liked") && (
          <div className="brutalist-playlists-view">
            <div className="suites-masthead">
              <h2>{currentView === "liked" ? "FAVORITE SOUNDSCAPES" : "MATRIX SUITES & ANTHOLOGIES"}</h2>
              <p>PHYSICAL CLUSTERS AND ALGORITHMIC DISPATCH MATRICES</p>
            </div>

            <div className="suites-bento-grid">
              {playlists.map((pl, idx) => (
                <div key={pl.id} className="suite-block-card" onClick={() => onPlayTrack(pl.tracks[0] || tracks[0])}>
                  <div className="suite-header">
                    <span className="suite-code">SUITE 0{idx + 1}</span>
                    <span className="suite-type">{pl.isSmartMix ? "AUTONOMOUS MATRIX" : "STATIC COLLECTION"}</span>
                  </div>
                  <div className="suite-art-well">
                    <img src={pl.coverArt} alt={pl.name} />
                  </div>
                  <div className="suite-body">
                    <h3>{pl.name}</h3>
                    <p>{pl.description}</p>
                    <div className="suite-footer">
                      <span>{pl.trackCount} COMPOSITIONS</span>
                      <button type="button" className="suite-trigger-btn">DISPATCH</button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* 5. MACHINE AUDIT // STATS */}
        {currentView === "stats" && (
          <div className="brutalist-stats-view">
            <div className="audit-masthead">
              <h2>MACHINE AUDIT // ACOUSTIC TELEMETRY</h2>
              <p>HARDWARE EXECUTION TELEMETRY AND LISTENING PROFILE LOGS</p>
            </div>

            <div className="audit-metrics-row">
              <div className="audit-card">
                <span className="audit-label">LIFETIME RUNTIME</span>
                <div className="audit-value">142.8 <small>HOURS</small></div>
                <div className="audit-progress-bar">
                  <div className="audit-fill" style={{ width: "78%" }} />
                </div>
                <span className="audit-sub">SYSTEM INTEGRITY: 100%</span>
              </div>

              <div className="audit-card">
                <span className="audit-label">PEAK TEMPO DENSITY</span>
                <div className="audit-value">124 <small>BPM</small></div>
                <div className="audit-progress-bar">
                  <div className="audit-fill" style={{ width: "62%" }} />
                </div>
                <span className="audit-sub">HARMONIC COHERENCE: HIGH</span>
              </div>

              <div className="audit-card">
                <span className="audit-label">FORMAT PURITY</span>
                <div className="audit-value">99.4 <small>%</small></div>
                <div className="audit-progress-bar">
                  <div className="audit-fill" style={{ width: "99.4%" }} />
                </div>
                <span className="audit-sub">BIT-PERFECT LOSSLESS FLAC</span>
              </div>
            </div>
          </div>
        )}

        {/* 6. SYSTEM CALIBRATION // SETTINGS */}
        {currentView === "settings" && (
          <div className="brutalist-settings-view">
            <div className="calibration-masthead">
              <h2>SYSTEM CALIBRATION // RUNTIME PARAMETERS</h2>
              <p>OPERATING SYSTEM AND LINUX AUDIO SUBSYSTEM HARDWARE PROTOCOLS</p>
            </div>

            <div className="calibration-chassis">
              <div className="cal-row">
                <div className="cal-info">
                  <strong>ALSA BIT-PERFECT DIRECT STREAM</strong>
                  <p>Bypass PulseAudio/PipeWire re-sampling for pure hardware DAC transmission.</p>
                </div>
                <span className="cal-state-badge active">BIT-PERFECT // ACTIVE</span>
              </div>

              <div className="cal-row">
                <div className="cal-info">
                  <strong>SYSTEM TRAY PERSISTENCE</strong>
                  <p>Close button minimizes application to Linux system tray and maintains audio playback.</p>
                </div>
                <span className="cal-state-badge active">MINIMIZE TO TRAY</span>
              </div>

              <div className="cal-row">
                <div className="cal-info">
                  <strong>CONFIGURED STORAGE ROOTS</strong>
                  <p>Local directories monitored by Kaze SQLite indexing service.</p>
                </div>
                <span className="cal-state-badge">4 DIRECTORIES SCANNED</span>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Persistent Horizon Audio Ribbon */}
      <footer className="model-b-player-ribbon">
        <div className="ribbon-track">
          <img src={currentTrack.coverArt} alt={currentTrack.title} className="ribbon-art" />
          <div className="ribbon-text">
            <div className="ribbon-title">{currentTrack.title}</div>
            <div className="ribbon-artist">{currentTrack.artist} • {currentTrack.album}</div>
          </div>
        </div>

        {/* Scrubber & Controls */}
        <div className="ribbon-transport-center">
          <div className="ribbon-buttons">
            <button
              type="button"
              className={isShuffled ? "ribbon-active-btn" : ""}
              onClick={onToggleShuffle}
              title="Shuffle"
            >
              <Shuffle size={14} />
            </button>
            <button type="button" onClick={onPrevTrack}><SkipBack size={16} /></button>
            <button type="button" className="ribbon-play-circle" onClick={onTogglePlayPause}>
              {isPlaying ? <Pause size={18} fill="#0f172a" /> : <Play size={18} fill="#0f172a" />}
            </button>
            <button type="button" onClick={onNextTrack}><SkipForward size={16} /></button>
            <button
              type="button"
              className={repeatMode !== "off" ? "ribbon-active-btn" : ""}
              onClick={onToggleRepeat}
              title={`Repeat: ${repeatMode}`}
            >
              <Repeat size={14} />
            </button>
            <button type="button" onClick={() => onToggleLike(currentTrack.id)}>
              <Heart size={16} fill={isLiked ? "#38bdf8" : "none"} color={isLiked ? "#38bdf8" : "currentColor"} />
            </button>
          </div>
          <div className="ribbon-progress-line">
            <span>{formatTime(playbackSecs)}</span>
            <div
              className="progress-line-track"
              onClick={(e) => {
                const rect = e.currentTarget.getBoundingClientRect();
                onSeek(Math.floor(((e.clientX - rect.left) / rect.width) * (currentTrack.durationSecs || 240)));
              }}
            >
              <div className="progress-line-fill" style={{ width: `${(playbackSecs / (currentTrack.durationSecs || 240)) * 100}%` }} />
            </div>
            <span>{formatTime(currentTrack.durationSecs || 240)}</span>
          </div>
        </div>

        <div className="ribbon-telemetry">
          <span className="telemetry-badge">96kHz MASTER</span>
          <div className="ribbon-vol-group">
            <Volume2 size={15} />
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={volume}
              onChange={(e) => onVolumeChange(parseFloat(e.target.value))}
              className="ribbon-vol-slider"
            />
          </div>
        </div>
      </footer>
    </div>
  );
};
