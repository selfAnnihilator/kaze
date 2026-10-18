import React, { useState, useEffect } from "react";
import { Sparkles, Eye, ChevronUp, ChevronDown, Wind, Newspaper, Image, RotateCcw } from "lucide-react";
import { DesignModelId } from "./types";
import { DESIGN_MODELS } from "./mockData";

interface DesignSwitcherProps {
  currentModel: DesignModelId;
  onSelectModel: (model: DesignModelId) => void;
}

export const DesignSwitcher: React.FC<DesignSwitcherProps> = ({
  currentModel,
  onSelectModel,
}) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const [isMinimized, setIsMinimized] = useState(false);

  // Keyboard shortcut listener: 1=Drift, 2=Broadsheet, 3=Gallery, 0=Standard
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger if typing in an input
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement ||
        (e.target as HTMLElement).isContentEditable
      ) {
        return;
      }

      if (e.key === "1") onSelectModel("drift");
      if (e.key === "2") onSelectModel("broadsheet");
      if (e.key === "3") onSelectModel("gallery");
      if (e.key === "0" || (e.key === "Escape" && currentModel !== "standard")) onSelectModel("standard");
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [currentModel, onSelectModel]);

  const activeMeta = DESIGN_MODELS.find((m) => m.id === currentModel);

  const modelIcons: Record<string, React.ReactNode> = {
    standard: <RotateCcw size={14} />,
    drift: <Wind size={14} />,
    broadsheet: <Newspaper size={14} />,
    gallery: <Image size={14} />,
  };

  if (isMinimized) {
    return (
      <button
        type="button"
        className="design-switcher-minimized-pill"
        onClick={() => setIsMinimized(false)}
        title="Open Design Exploration Switcher (Keys: 1, 2, 3, 0)"
      >
        <Sparkles size={14} className="sparkle-icon" />
        <span>Design Models</span>
        <ChevronUp size={12} />
      </button>
    );
  }

  return (
    <div className={`design-switcher-dock ${isExpanded ? "expanded" : ""}`}>
      {/* Top info drawer (shows when expanded) */}
      {isExpanded && activeMeta && (
        <div className="design-switcher-drawer">
          <div className="drawer-header">
            <div className="drawer-tag">{activeMeta.codename}</div>
            <div className="drawer-theme">Theme: {activeMeta.theme}</div>
          </div>
          <div className="drawer-tagline">{activeMeta.tagline}</div>
          <div className="drawer-description">{activeMeta.layoutDescription}</div>
          <div className="drawer-tip">
            Keyboard shortcuts: Press <strong>1</strong> (Drift), <strong>2</strong> (Broadsheet),{" "}
            <strong>3</strong> (Gallery), or <strong>0</strong> (Standard Kaze).
          </div>
        </div>
      )}

      {/* Main floating control capsule */}
      <div className="design-switcher-bar">
        <div className="switcher-brand">
          <Sparkles size={15} style={{ color: activeMeta?.accent || "#D4883A" }} />
          <span className="brand-text">Kaze Lab</span>
        </div>

        <div className="switcher-options">
          <a className="switcher-btn" href="?lab=spatial" style={{ textDecoration: "none", color: "#f5b272" }}>
            <Wind size={14} />
            <span className="btn-label">Spatial design</span>
          </a>
          <a className="switcher-btn" href="?lab=brutalist" style={{ textDecoration: "none", color: "#f5b272" }}>
            <Newspaper size={14} />
            <span className="btn-label">Brutalist design</span>
          </a>
          {/* Production Standard Kaze */}
          <button
            type="button"
            className={`switcher-btn ${currentModel === "standard" ? "active" : ""}`}
            onClick={() => onSelectModel("standard")}
            title="Standard Kaze Production UI (Key: 0)"
          >
            {modelIcons.standard}
            <span className="btn-label">Standard</span>
            <span className="shortcut-badge">0</span>
          </button>

          {/* Model Options */}
          {DESIGN_MODELS.map((model) => {
            const isActive = currentModel === model.id;
            return (
              <button
                key={model.id}
                type="button"
                className={`switcher-btn ${isActive ? "active" : ""}`}
                style={isActive ? { borderColor: `${model.accent}88`, boxShadow: `0 0 16px ${model.accent}33` } : {}}
                onClick={() => onSelectModel(model.id)}
                title={`${model.name} — ${model.tagline} (Key: ${model.keyShortcut})`}
              >
                {modelIcons[model.id]}
                <span className="btn-label">{model.name}</span>
                <span className="shortcut-badge">{model.keyShortcut}</span>
              </button>
            );
          })}
        </div>

        {/* Info Toggle & Minimize Actions */}
        <div className="switcher-actions">
          {activeMeta && (
            <button
              type="button"
              className="action-icon-btn"
              onClick={() => setIsExpanded(!isExpanded)}
              title={isExpanded ? "Collapse concept specs" : "View concept philosophy & specs"}
            >
              {isExpanded ? <ChevronDown size={14} /> : <Eye size={14} />}
            </button>
          )}

          <button
            type="button"
            className="action-icon-btn minimize"
            onClick={() => setIsMinimized(true)}
            title="Minimize switcher"
          >
            <ChevronDown size={14} />
          </button>
        </div>
      </div>
    </div>
  );
};
