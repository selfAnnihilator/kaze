import React, { useState, useEffect, useCallback, useRef } from "react";
import {
  Calendar,
  Activity,
  Award,
  Music,
  Users,
  Play,
  Flame,
  Archive,
  RotateCw,
  ChevronDown,
  Camera,
  Trash2,
  User,
  Loader2,
  AlertCircle,
  Pencil,
} from "lucide-react";
import { DailyListeningPoint, StatsOverview, UserProfile } from "../../types";
import { dispatchCommand } from "../../services/api";

const MONTH_NAMES = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

interface StatsViewProps {
  currentUser: UserProfile | null;
  onPlayTrack?: (trackId: string) => void;
  onOpenAuthModal?: () => void;
  fetchStatsOverview: (year?: number, month?: number) => Promise<StatsOverview | null>;
  refreshTrigger?: number;
  onUpdateUser?: (user: UserProfile) => void;
}

export const StatsView: React.FC<StatsViewProps> = ({
  currentUser,
  onPlayTrack,
  onOpenAuthModal,
  fetchStatsOverview,
  refreshTrigger,
  onUpdateUser,
}) => {
  const [stats, setStats] = useState<StatsOverview | null>(null);
  const [selectedYear, setSelectedYear] = useState<number | undefined>(undefined);
  const [selectedMonth, setSelectedMonth] = useState<number | undefined>(undefined);
  const [hoveredDay, setHoveredDay] = useState<DailyListeningPoint | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isUploadingAvatar, setIsUploadingAvatar] = useState(false);
  const [avatarError, setAvatarError] = useState<string | null>(null);
  const [showAvatarMenu, setShowAvatarMenu] = useState(false);
  const avatarContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      if (avatarContainerRef.current && !avatarContainerRef.current.contains(e.target as Node)) {
        setShowAvatarMenu(false);
      }
    };
    document.addEventListener("mousedown", handleOutsideClick);
    return () => {
      document.removeEventListener("mousedown", handleOutsideClick);
    };
  }, []);

  const handlePickAvatar = async () => {
    if (!currentUser) {
      onOpenAuthModal?.();
      return;
    }
    setAvatarError(null);
    setIsUploadingAvatar(true);
    try {
      const res = await dispatchCommand({ command: "UploadAvatar" });
      if (res?.data) {
        onUpdateUser?.(res.data);
      }
    } catch (err: any) {
      console.error("Failed to upload avatar:", err);
      const msg = typeof err === "string" ? err : err?.message || "Failed to upload avatar.";
      setAvatarError(msg);
    } finally {
      setIsUploadingAvatar(false);
    }
  };

  const handleRemoveAvatar = async () => {
    if (!currentUser) return;
    setAvatarError(null);
    setIsUploadingAvatar(true);
    try {
      const res = await dispatchCommand({ command: "RemoveAvatar" });
      if (res?.data) {
        onUpdateUser?.(res.data);
      }
    } catch (err: any) {
      console.error("Failed to remove avatar:", err);
      const msg = typeof err === "string" ? err : err?.message || "Failed to remove avatar.";
      setAvatarError(msg);
    } finally {
      setIsUploadingAvatar(false);
    }
  };

  const loadStats = useCallback(
    async (manual = false) => {
      if (manual) {
        setIsRefreshing(true);
      } else {
        setIsLoading(true);
      }
      try {
        const data = await fetchStatsOverview(selectedYear, selectedMonth);
        setStats(data);
        if (data && selectedMonth === undefined) {
          setSelectedMonth(data.selected_month);
        }
      } catch (err) {
        console.error("Failed to load stats:", err);
      } finally {
        setIsLoading(false);
        setIsRefreshing(false);
      }
    },
    [selectedYear, selectedMonth, fetchStatsOverview]
  );

  useEffect(() => {
    loadStats();
  }, [loadStats, refreshTrigger]);

  const formatSeconds = (totalSecs: number): string => {
    if (!totalSecs || totalSecs <= 0) return "0m";
    const hours = Math.floor(totalSecs / 3600);
    const minutes = Math.floor((totalSecs % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  };

  const formatDate = (timestampSecs: number): string => {
    if (!timestampSecs || timestampSecs <= 0) return "Recently";
    return new Date(timestampSecs * 1000).toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  };

  const currentYear = stats?.current_year || new Date().getFullYear();
  const viewingYear = selectedYear ?? currentYear;
  const isPastYear = viewingYear < currentYear;

  const activeMonthNum = selectedMonth ?? stats?.selected_month ?? (new Date().getMonth() + 1);
  const activeMonthName = MONTH_NAMES[activeMonthNum - 1] || "Month";

  const monthlyGraph = stats?.monthly_graph || [];
  const maxGraphSeconds = Math.max(...monthlyGraph.map((p) => p.total_seconds), 1);
  const daysWithMusic = monthlyGraph.filter((p) => p.total_seconds > 0).length;
  const dailyAvgSecs =
    monthlyGraph.length > 0 ? Math.round((stats?.monthly_seconds || 0) / monthlyGraph.length) : 0;

  return (
    <div
      style={{
        padding: "32px 40px 100px",
        maxWidth: "1200px",
        margin: "0 auto",
        color: "#fff",
      }}
    >
      {/* Profile Section Header Card */}
      <div
        style={{
          background: "linear-gradient(135deg, rgba(30, 27, 75, 0.45) 0%, rgba(15, 23, 42, 0.65) 100%)",
          border: "1px solid rgba(255, 255, 255, 0.08)",
          borderRadius: "20px",
          padding: "24px 28px",
          marginBottom: "36px",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          flexWrap: "wrap",
          gap: "20px",
          boxShadow: "0 8px 32px -8px rgba(0, 0, 0, 0.5)",
          backdropFilter: "blur(12px)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "20px", flexWrap: "wrap" }}>
          {/* Profile Photo / Avatar with Bottom-Left Pencil Edit Trigger */}
          <div style={{ position: "relative", width: "72px", height: "72px" }} ref={avatarContainerRef}>
            <div
              onClick={() => {
                if (!currentUser) {
                  onOpenAuthModal?.();
                  return;
                }
                if (currentUser.avatar_data_url) {
                  setShowAvatarMenu((prev) => !prev);
                } else {
                  handlePickAvatar();
                }
              }}
              title={currentUser ? (currentUser.avatar_data_url ? "Click to manage photo" : "Click to upload photo") : undefined}
              style={{
                width: "72px",
                height: "72px",
                borderRadius: "50%",
                background: currentUser?.avatar_data_url
                  ? "transparent"
                  : "linear-gradient(135deg, #7c3aed 0%, #ec4899 100%)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: "28px",
                fontWeight: 800,
                color: "#fff",
                border: "3px solid rgba(168, 85, 247, 0.5)",
                boxShadow: "0 0 20px rgba(168, 85, 247, 0.25)",
                overflow: "hidden",
                cursor: currentUser ? "pointer" : "default",
                position: "relative",
                flexShrink: 0,
              }}
            >
              {currentUser?.avatar_data_url ? (
                <img
                  src={currentUser.avatar_data_url}
                  alt={currentUser.username}
                  style={{
                    width: "100%",
                    height: "100%",
                    objectFit: "cover",
                  }}
                />
              ) : currentUser ? (
                <span>{currentUser.username[0].toUpperCase()}</span>
              ) : (
                <User size={36} color="rgba(255, 255, 255, 0.5)" />
              )}

              {/* Uploading overlay */}
              {isUploadingAvatar && (
                <div
                  style={{
                    position: "absolute",
                    inset: 0,
                    background: "rgba(0, 0, 0, 0.75)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                  }}
                >
                  <Loader2 size={24} className="animate-spin" color="#c084fc" />
                </div>
              )}
            </div>

            {/* Pencil Icon Edit Button at Bottom-Left of Profile Circle */}
            {currentUser && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  if (currentUser.avatar_data_url) {
                    setShowAvatarMenu((prev) => !prev);
                  } else {
                    handlePickAvatar();
                  }
                }}
                disabled={isUploadingAvatar}
                title={currentUser.avatar_data_url ? "Edit or remove profile photo" : "Upload profile photo"}
                style={{
                  position: "absolute",
                  bottom: "-2px",
                  left: "-2px",
                  width: "28px",
                  height: "28px",
                  borderRadius: "50%",
                  background: "#8b5cf6",
                  border: "2.5px solid #141328",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  cursor: isUploadingAvatar ? "not-allowed" : "pointer",
                  color: "#fff",
                  boxShadow: "0 2px 8px rgba(0, 0, 0, 0.5)",
                  transition: "all 0.15s ease",
                  padding: 0,
                  zIndex: 10,
                }}
                onMouseEnter={(e) => {
                  if (!isUploadingAvatar) e.currentTarget.style.background = "#7c3aed";
                }}
                onMouseLeave={(e) => {
                  if (!isUploadingAvatar) e.currentTarget.style.background = "#8b5cf6";
                }}
              >
                {isUploadingAvatar ? (
                  <Loader2 size={13} className="animate-spin" />
                ) : (
                  <Pencil size={13} strokeWidth={2.5} />
                )}
              </button>
            )}

            {/* Dropdown menu when avatar exists */}
            {showAvatarMenu && currentUser?.avatar_data_url && (
              <div
                style={{
                  position: "absolute",
                  top: "100%",
                  left: 0,
                  marginTop: "8px",
                  background: "#1e1b4b",
                  border: "1px solid rgba(168, 85, 247, 0.35)",
                  borderRadius: "12px",
                  padding: "6px",
                  zIndex: 50,
                  minWidth: "160px",
                  boxShadow: "0 10px 25px -5px rgba(0, 0, 0, 0.6)",
                  display: "flex",
                  flexDirection: "column",
                  gap: "4px",
                }}
              >
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setShowAvatarMenu(false);
                    handlePickAvatar();
                  }}
                  disabled={isUploadingAvatar}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "8px",
                    background: "transparent",
                    border: "none",
                    color: "#fff",
                    padding: "8px 10px",
                    borderRadius: "8px",
                    fontSize: "12px",
                    fontWeight: 600,
                    cursor: "pointer",
                    textAlign: "left",
                    transition: "background 0.15s ease",
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.background = "rgba(168, 85, 247, 0.2)")}
                  onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
                >
                  <Camera size={14} color="#c084fc" />
                  <span>Change photo</span>
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setShowAvatarMenu(false);
                    handleRemoveAvatar();
                  }}
                  disabled={isUploadingAvatar}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "8px",
                    background: "transparent",
                    border: "none",
                    color: "#f87171",
                    padding: "8px 10px",
                    borderRadius: "8px",
                    fontSize: "12px",
                    fontWeight: 600,
                    cursor: "pointer",
                    textAlign: "left",
                    transition: "background 0.15s ease",
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.background = "rgba(239, 68, 68, 0.15)")}
                  onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
                >
                  <Trash2 size={14} color="#f87171" />
                  <span>Remove photo</span>
                </button>
              </div>
            )}
          </div>

          {/* User Details */}
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: "12px", marginBottom: "6px" }}>
              <h2
                style={{
                  fontSize: "26px",
                  fontWeight: 800,
                  margin: 0,
                  color: "#fff",
                  letterSpacing: "-0.5px",
                }}
              >
                {currentUser ? currentUser.username : "Guest Listener"}
              </h2>
            </div>

            <div
              style={{
                fontSize: "13px",
                color: "rgba(255, 255, 255, 0.65)",
                display: "flex",
                alignItems: "center",
                gap: "8px",
                flexWrap: "wrap",
              }}
            >
              {currentUser ? (
                <span>
                  Member since{" "}
                  <strong style={{ color: "#fff" }}>
                    {formatDate(currentUser.created_at)}
                  </strong>
                </span>
              ) : (
                <span>
                  Listening history saved locally •{" "}
                  <button
                    onClick={onOpenAuthModal}
                    style={{
                      background: "none",
                      border: "none",
                      color: "#a78bfa",
                      cursor: "pointer",
                      textDecoration: "underline",
                      padding: 0,
                      fontSize: "13px",
                      fontWeight: 600,
                    }}
                  >
                    Sign in to customize profile photo
                  </button>
                </span>
              )}
            </div>

            {avatarError && (
              <div
                style={{
                  marginTop: "8px",
                  color: "#f87171",
                  fontSize: "12px",
                  display: "flex",
                  alignItems: "center",
                  gap: "6px",
                }}
              >
                <AlertCircle size={14} />
                <span>{avatarError}</span>
              </div>
            )}
          </div>
        </div>

        {/* Guest Auth Button (No external buttons when logged in) */}
        {!currentUser && (
          <button
            onClick={onOpenAuthModal}
            style={{
              background: "linear-gradient(135deg, #7c3aed, #6366f1)",
              border: "none",
              color: "#fff",
              padding: "8px 16px",
              borderRadius: "10px",
              fontSize: "13px",
              fontWeight: 600,
              cursor: "pointer",
            }}
          >
            Sign In / Register
          </button>
        )}
      </div>

      {/* Listening Stats Header Banner */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          flexWrap: "wrap",
          gap: "20px",
          marginBottom: "32px",
        }}
      >
        <div>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "10px",
              marginBottom: "8px",
            }}
          >
            <h1
              style={{
                fontSize: "28px",
                fontWeight: 800,
                margin: 0,
                letterSpacing: "-0.5px",
              }}
            >
              Listening Stats
            </h1>
            <span
              style={{
                background: "linear-gradient(135deg, rgba(139, 92, 246, 0.2), rgba(99, 102, 241, 0.2))",
                border: "1px solid rgba(139, 92, 246, 0.4)",
                color: "#c084fc",
                padding: "2px 10px",
                borderRadius: "9999px",
                fontSize: "12px",
                fontWeight: 700,
              }}
            >
              {viewingYear}
            </span>
          </div>

          <div
            style={{
              fontSize: "13px",
              color: "rgba(255, 255, 255, 0.6)",
            }}
          >
            Overview of your listening activity, trends, and top records
          </div>
        </div>

        {/* Header Right: Year Selector Tabs & Manual Refresh */}
        <div style={{ display: "flex", alignItems: "center", gap: "10px", flexWrap: "wrap" }}>
          {stats && stats.available_years && stats.available_years.length > 0 && (
            <div
              style={{
                display: "flex",
                gap: "6px",
                background: "rgba(255, 255, 255, 0.05)",
                padding: "4px",
                borderRadius: "12px",
                border: "1px solid rgba(255, 255, 255, 0.08)",
              }}
            >
              {stats.available_years.map((year) => {
                const isActive = (selectedYear ?? currentYear) === year;
                return (
                  <button
                    key={year}
                    onClick={() => setSelectedYear(year === currentYear ? undefined : year)}
                    style={{
                      padding: "6px 14px",
                      borderRadius: "8px",
                      border: "none",
                      background: isActive
                        ? "linear-gradient(135deg, #8b5cf6, #6366f1)"
                        : "transparent",
                      color: isActive ? "#fff" : "rgba(255, 255, 255, 0.6)",
                      fontWeight: isActive ? 700 : 500,
                      fontSize: "13px",
                      cursor: "pointer",
                      transition: "all 0.15s ease",
                      display: "flex",
                      alignItems: "center",
                      gap: "6px",
                    }}
                  >
                    {year < currentYear && <Archive size={14} />}
                    <span>{year === currentYear ? `${year} (Current)` : year}</span>
                  </button>
                );
              })}
            </div>
          )}

          <button
            onClick={() => loadStats(true)}
            disabled={isLoading || isRefreshing}
            style={{
              padding: "7px 14px",
              borderRadius: "10px",
              border: "1px solid rgba(255, 255, 255, 0.12)",
              background: "rgba(255, 255, 255, 0.06)",
              color: "#fff",
              fontSize: "12px",
              fontWeight: 600,
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              gap: "7px",
              transition: "all 0.15s ease",
            }}
            title="Refresh listening stats"
          >
            <RotateCw size={14} className={isRefreshing ? "spin-animation" : ""} />
            <span>Refresh</span>
          </button>
        </div>
      </div>

      {isLoading ? (
        <div
          style={{
            padding: "80px 0",
            textAlign: "center",
            color: "rgba(255, 255, 255, 0.5)",
          }}
        >
          <div
            className="spinner"
            style={{
              width: "36px",
              height: "36px",
              border: "3px solid rgba(139, 92, 246, 0.2)",
              borderTopColor: "#8b5cf6",
              borderRadius: "50%",
              margin: "0 auto 16px",
              animation: "spin 1s linear infinite",
            }}
          />
          <p>Loading your listening records...</p>
        </div>
      ) : !isPastYear ? (
        <>
          {/* Row 1: Daily and Weekly Section (2-Column Grid) */}
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))",
              gap: "20px",
              marginBottom: "20px",
            }}
          >
            {/* Daily Music Time Card */}
            <div
              style={{
                background: "linear-gradient(135deg, rgba(30, 27, 75, 0.6) 0%, rgba(24, 24, 27, 0.8) 100%)",
                border: "1px solid rgba(139, 92, 246, 0.2)",
                borderRadius: "18px",
                padding: "20px 24px",
                position: "relative",
                overflow: "hidden",
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  marginBottom: "12px",
                }}
              >
                <span
                  style={{
                    fontSize: "13px",
                    fontWeight: 600,
                    color: "rgba(255, 255, 255, 0.6)",
                    textTransform: "uppercase",
                    letterSpacing: "0.5px",
                  }}
                >
                  Daily Music Time
                </span>
                <div
                  style={{
                    width: "36px",
                    height: "36px",
                    borderRadius: "10px",
                    background: "rgba(236, 72, 153, 0.15)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    color: "#f472b6",
                  }}
                >
                  <Flame size={20} />
                </div>
              </div>
              <div style={{ fontSize: "32px", fontWeight: 800, color: "#fff" }}>
                {formatSeconds(stats?.daily_seconds || 0)}
              </div>
              <div
                style={{
                  fontSize: "12px",
                  color: "rgba(255, 255, 255, 0.4)",
                  marginTop: "4px",
                }}
              >
                Today's listening session
              </div>
            </div>

            {/* Weekly Music Time Card */}
            <div
              style={{
                background: "linear-gradient(135deg, rgba(20, 35, 60, 0.6) 0%, rgba(24, 24, 27, 0.8) 100%)",
                border: "1px solid rgba(59, 130, 246, 0.2)",
                borderRadius: "18px",
                padding: "20px 24px",
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  marginBottom: "12px",
                }}
              >
                <span
                  style={{
                    fontSize: "13px",
                    fontWeight: 600,
                    color: "rgba(255, 255, 255, 0.6)",
                    textTransform: "uppercase",
                    letterSpacing: "0.5px",
                  }}
                >
                  Weekly Music Time
                </span>
                <div
                  style={{
                    width: "36px",
                    height: "36px",
                    borderRadius: "10px",
                    background: "rgba(59, 130, 246, 0.15)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    color: "#60a5fa",
                  }}
                >
                  <Calendar size={20} />
                </div>
              </div>
              <div style={{ fontSize: "32px", fontWeight: 800, color: "#fff" }}>
                {formatSeconds(stats?.weekly_seconds || 0)}
              </div>
              <div
                style={{
                  fontSize: "12px",
                  color: "rgba(255, 255, 255, 0.4)",
                  marginTop: "4px",
                }}
              >
                Past 7 days
              </div>
            </div>
          </div>

          {/* Row 2: Monthly Stats Section (Takes Whole Width Below Daily & Weekly) */}
          <div
            style={{
              background: "linear-gradient(135deg, rgba(16, 50, 40, 0.55) 0%, rgba(24, 24, 27, 0.85) 100%)",
              border: "1px solid rgba(16, 185, 129, 0.25)",
              borderRadius: "20px",
              padding: "24px 28px",
              marginBottom: "36px",
              width: "100%",
              boxSizing: "border-box",
              position: "relative",
            }}
          >
            {/* Monthly Card Top Header: Time on Left, Month Name and Month Selector on Top Right */}
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "flex-start",
                flexWrap: "wrap",
                gap: "16px",
                marginBottom: "20px",
              }}
            >
              {/* Left: Monthly Music Time Title and Main Time Display */}
              <div>
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "8px",
                    marginBottom: "6px",
                  }}
                >
                  <div
                    style={{
                      width: "32px",
                      height: "32px",
                      borderRadius: "8px",
                      background: "rgba(16, 185, 129, 0.15)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      color: "#34d399",
                    }}
                  >
                    <Activity size={18} />
                  </div>
                  <span
                    style={{
                      fontSize: "13px",
                      fontWeight: 700,
                      color: "rgba(255, 255, 255, 0.7)",
                      textTransform: "uppercase",
                      letterSpacing: "0.5px",
                    }}
                  >
                    Monthly Music Time
                  </span>
                </div>

                <div style={{ display: "flex", alignItems: "baseline", gap: "12px", flexWrap: "wrap" }}>
                  <span
                    style={{
                      fontSize: "36px",
                      fontWeight: 800,
                      color: "#fff",
                      letterSpacing: "-0.5px",
                    }}
                  >
                    {formatSeconds(stats?.monthly_seconds || 0)}
                  </span>
                  <span style={{ fontSize: "13px", color: "rgba(255, 255, 255, 0.5)" }}>
                    total in {activeMonthName} {viewingYear}
                  </span>
                </div>
              </div>

              {/* Right: Month Name Badge and Option to Choose the Month */}
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "12px",
                  flexWrap: "wrap",
                }}
              >
                {/* Active Month Name Badge */}
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "6px",
                    background: "rgba(16, 185, 129, 0.15)",
                    border: "1px solid rgba(16, 185, 129, 0.35)",
                    color: "#34d399",
                    padding: "6px 14px",
                    borderRadius: "9999px",
                    fontSize: "13px",
                    fontWeight: 700,
                  }}
                >
                  <Calendar size={14} />
                  <span>{activeMonthName}</span>
                </div>

                {/* Option to Choose Month Dropdown */}
                <div style={{ position: "relative" }}>
                  <select
                    value={activeMonthNum}
                    onChange={(e) => {
                      const m = parseInt(e.target.value, 10);
                      setSelectedMonth(m);
                    }}
                    style={{
                      appearance: "none",
                      backgroundColor: "rgba(255, 255, 255, 0.08)",
                      border: "1px solid rgba(255, 255, 255, 0.18)",
                      borderRadius: "10px",
                      color: "#fff",
                      padding: "7px 34px 7px 14px",
                      fontSize: "13px",
                      fontWeight: 600,
                      cursor: "pointer",
                      outline: "none",
                      transition: "border-color 0.15s ease",
                    }}
                  >
                    {MONTH_NAMES.map((mName, idx) => {
                      const mNum = idx + 1;
                      return (
                        <option
                          key={mNum}
                          value={mNum}
                          style={{ backgroundColor: "#18181b", color: "#fff" }}
                        >
                          {mName}
                        </option>
                      );
                    })}
                  </select>
                  <ChevronDown
                    size={14}
                    style={{
                      position: "absolute",
                      right: "12px",
                      top: "50%",
                      transform: "translateY(-50%)",
                      color: "rgba(255, 255, 255, 0.6)",
                      pointerEvents: "none",
                    }}
                  />
                </div>
              </div>
            </div>

            {/* 30-Day Listening Activity Graph */}
            <div
              style={{
                background: "rgba(0, 0, 0, 0.25)",
                borderRadius: "14px",
                padding: "18px 20px 14px",
                border: "1px solid rgba(255, 255, 255, 0.06)",
              }}
            >
              {/* Summary line above graph */}
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: "16px",
                  fontSize: "12px",
                  color: "rgba(255, 255, 255, 0.6)",
                  minHeight: "22px",
                }}
              >
                <div>
                  {hoveredDay ? (
                    <span style={{ color: "#fff" }}>
                      📅 <strong style={{ color: "#34d399" }}>{hoveredDay.date}</strong> (Day {hoveredDay.day}):{" "}
                      <strong>{formatSeconds(hoveredDay.total_seconds)}</strong>
                    </span>
                  ) : (
                    <span>Daily listening activity across {activeMonthName} ({monthlyGraph.length} days)</span>
                  )}
                </div>

                <div style={{ display: "flex", gap: "16px" }}>
                  <span>
                    Active days: <strong style={{ color: "#fff" }}>{daysWithMusic}</strong>/{monthlyGraph.length}
                  </span>
                  <span>
                    Daily avg: <strong style={{ color: "#fff" }}>{formatSeconds(dailyAvgSecs)}</strong>
                  </span>
                </div>
              </div>

              {/* Bars container */}
              <div
                style={{
                  display: "flex",
                  alignItems: "flex-end",
                  gap: "4px",
                  height: "130px",
                  width: "100%",
                  paddingBottom: "8px",
                  borderBottom: "1px solid rgba(255, 255, 255, 0.1)",
                }}
              >
                {monthlyGraph.map((point) => {
                  const isHovered = hoveredDay?.day === point.day;
                  const hasTime = point.total_seconds > 0;
                  const heightPct = hasTime
                    ? Math.max(Math.round((point.total_seconds / maxGraphSeconds) * 100), 8)
                    : 4;

                  return (
                    <div
                      key={point.day}
                      onMouseEnter={() => setHoveredDay(point)}
                      onMouseLeave={() => setHoveredDay(null)}
                      style={{
                        flex: 1,
                        height: "100%",
                        display: "flex",
                        flexDirection: "column",
                        justifyContent: "flex-end",
                        alignItems: "center",
                        cursor: "pointer",
                        position: "relative",
                      }}
                    >
                      {/* Tooltip on hover */}
                      {isHovered && (
                        <div
                          style={{
                            position: "absolute",
                            bottom: "calc(100% + 8px)",
                            background: "rgba(24, 24, 27, 0.95)",
                            border: "1px solid rgba(16, 185, 129, 0.4)",
                            color: "#fff",
                            padding: "6px 10px",
                            borderRadius: "8px",
                            fontSize: "11px",
                            fontWeight: 600,
                            whiteSpace: "nowrap",
                            zIndex: 20,
                            boxShadow: "0 8px 20px rgba(0, 0, 0, 0.5)",
                            pointerEvents: "none",
                          }}
                        >
                          <div>Day {point.day} ({point.date})</div>
                          <div style={{ color: "#34d399", fontWeight: 700 }}>
                            {formatSeconds(point.total_seconds)}
                          </div>
                        </div>
                      )}

                      {/* Bar element */}
                      <div
                        style={{
                          width: "100%",
                          maxWidth: "24px",
                          height: `${heightPct}%`,
                          borderRadius: "4px 4px 0 0",
                          background: isHovered
                            ? "linear-gradient(180deg, #6ee7b7 0%, #10b981 100%)"
                            : hasTime
                            ? "linear-gradient(180deg, #34d399 0%, #059669 100%)"
                            : "rgba(255, 255, 255, 0.07)",
                          boxShadow: isHovered
                            ? "0 0 12px rgba(52, 211, 153, 0.6)"
                            : hasTime
                            ? "0 0 4px rgba(52, 211, 153, 0.2)"
                            : "none",
                          transition: "all 0.15s ease",
                        }}
                      />
                    </div>
                  );
                })}
              </div>

              {/* Day markers along X-Axis */}
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  paddingTop: "6px",
                  fontSize: "11px",
                  color: "rgba(255, 255, 255, 0.4)",
                  fontWeight: 600,
                }}
              >
                <span>Day 1</span>
                <span>Day 5</span>
                <span>Day 10</span>
                <span>Day 15</span>
                <span>Day 20</span>
                <span>Day 25</span>
                <span>Day {monthlyGraph.length}</span>
              </div>
            </div>
          </div>

          {/* Grid Layout: Top 10 Songs & Top 10 Artists */}
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(480px, 1fr))",
              gap: "28px",
              marginBottom: "36px",
            }}
          >
            {/* Top 10 Songs */}
            <div
              style={{
                background: "#18181b",
                border: "1px solid rgba(255, 255, 255, 0.08)",
                borderRadius: "16px",
                padding: "24px",
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "10px",
                  marginBottom: "20px",
                }}
              >
                <div
                  style={{
                    width: "32px",
                    height: "32px",
                    borderRadius: "8px",
                    background: "rgba(139, 92, 246, 0.15)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    color: "#a78bfa",
                  }}
                >
                  <Music size={18} />
                </div>
                <div>
                  <h2 style={{ fontSize: "18px", fontWeight: 700, margin: 0 }}>
                    Top 10 Songs
                  </h2>
                  <p
                    style={{
                      margin: 0,
                      fontSize: "12px",
                      color: "rgba(255, 255, 255, 0.4)",
                    }}
                  >
                    Ranked by play score and listen duration
                  </p>
                </div>
              </div>

              {!stats?.top_songs || stats.top_songs.length === 0 ? (
                <div
                  style={{
                    padding: "36px 0",
                    textAlign: "center",
                    color: "rgba(255, 255, 255, 0.4)",
                    fontSize: "13px",
                  }}
                >
                  No songs listened to yet this year. Play some music to see your top songs!
                </div>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                  {stats.top_songs.map((song, idx) => {
                    const rank = idx + 1;
                    const rankColor =
                      rank === 1 ? "#fbbf24" : rank === 2 ? "#e2e8f0" : rank === 3 ? "#d97706" : "rgba(255, 255, 255, 0.5)";
                    return (
                      <div
                        key={song.track_id}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          padding: "10px 14px",
                          borderRadius: "10px",
                          background: rank <= 3 ? "rgba(255, 255, 255, 0.03)" : "transparent",
                          border: rank <= 3 ? "1px solid rgba(255, 255, 255, 0.05)" : "none",
                          transition: "background 0.15s ease",
                        }}
                        className="stats-item-row"
                      >
                        <span
                          style={{
                            width: "28px",
                            fontSize: "15px",
                            fontWeight: 800,
                            color: rankColor,
                            textAlign: "center",
                            flexShrink: 0,
                          }}
                        >
                          {rank}
                        </span>

                        <div style={{ flex: 1, minWidth: 0, margin: "0 14px" }}>
                          <div
                            style={{
                              fontSize: "14px",
                              fontWeight: 600,
                              color: "#fff",
                              overflow: "hidden",
                              textOverflow: "ellipsis",
                              whiteSpace: "nowrap",
                            }}
                          >
                            {song.title}
                          </div>
                          <div
                            style={{
                              fontSize: "12px",
                              color: "rgba(255, 255, 255, 0.5)",
                              overflow: "hidden",
                              textOverflow: "ellipsis",
                              whiteSpace: "nowrap",
                            }}
                          >
                            {song.artist_name || "Unknown Artist"}
                          </div>
                        </div>

                        <div style={{ textAlign: "right", marginRight: "12px" }}>
                          <div style={{ fontSize: "13px", fontWeight: 600, color: "#fff" }}>
                            {formatSeconds(song.total_seconds)}
                          </div>
                          <div style={{ fontSize: "11px", color: "rgba(255, 255, 255, 0.4)" }}>
                            {song.play_count} {song.play_count === 1 ? "play" : "plays"}
                          </div>
                        </div>

                        {onPlayTrack && (
                          <button
                            onClick={() => onPlayTrack(song.track_id)}
                            title={`Play ${song.title}`}
                            style={{
                              width: "32px",
                              height: "32px",
                              borderRadius: "50%",
                              background: "rgba(139, 92, 246, 0.2)",
                              border: "none",
                              color: "#c084fc",
                              display: "flex",
                              alignItems: "center",
                              justifyContent: "center",
                              cursor: "pointer",
                              flexShrink: 0,
                            }}
                          >
                            <Play size={14} fill="currentColor" />
                          </button>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

            {/* Top 10 Artists */}
            <div
              style={{
                background: "#18181b",
                border: "1px solid rgba(255, 255, 255, 0.08)",
                borderRadius: "16px",
                padding: "24px",
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "10px",
                  marginBottom: "20px",
                }}
              >
                <div
                  style={{
                    width: "32px",
                    height: "32px",
                    borderRadius: "8px",
                    background: "rgba(236, 72, 153, 0.15)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    color: "#f472b6",
                  }}
                >
                  <Users size={18} />
                </div>
                <div>
                  <h2 style={{ fontSize: "18px", fontWeight: 700, margin: 0 }}>
                    Top 10 Artists
                  </h2>
                  <p
                    style={{
                      margin: 0,
                      fontSize: "12px",
                      color: "rgba(255, 255, 255, 0.4)",
                    }}
                  >
                    Your most listened to artists
                  </p>
                </div>
              </div>

              {!stats?.top_artists || stats.top_artists.length === 0 ? (
                <div
                  style={{
                    padding: "36px 0",
                    textAlign: "center",
                    color: "rgba(255, 255, 255, 0.4)",
                    fontSize: "13px",
                  }}
                >
                  No artists recorded yet this year.
                </div>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                  {stats.top_artists.map((artist, idx) => {
                    const rank = idx + 1;
                    const rankColor =
                      rank === 1 ? "#fbbf24" : rank === 2 ? "#e2e8f0" : rank === 3 ? "#d97706" : "rgba(255, 255, 255, 0.5)";
                    return (
                      <div
                        key={artist.artist_id}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          padding: "10px 14px",
                          borderRadius: "10px",
                          background: rank <= 3 ? "rgba(255, 255, 255, 0.03)" : "transparent",
                          border: rank <= 3 ? "1px solid rgba(255, 255, 255, 0.05)" : "none",
                        }}
                      >
                        <span
                          style={{
                            width: "28px",
                            fontSize: "15px",
                            fontWeight: 800,
                            color: rankColor,
                            textAlign: "center",
                            flexShrink: 0,
                          }}
                        >
                          {rank}
                        </span>

                        <div style={{ flex: 1, minWidth: 0, margin: "0 14px" }}>
                          <div
                            style={{
                              fontSize: "14px",
                              fontWeight: 600,
                              color: "#fff",
                              overflow: "hidden",
                              textOverflow: "ellipsis",
                              whiteSpace: "nowrap",
                            }}
                          >
                            {artist.name}
                          </div>
                        </div>

                        <div style={{ textAlign: "right" }}>
                          <div style={{ fontSize: "13px", fontWeight: 600, color: "#fff" }}>
                            {formatSeconds(artist.total_seconds)}
                          </div>
                          <div style={{ fontSize: "11px", color: "rgba(255, 255, 255, 0.4)" }}>
                            {artist.play_count} {artist.play_count === 1 ? "play" : "plays"}
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          </div>

          {/* Top 5 Days User Listened to Most */}
          <div
            style={{
              background: "#18181b",
              border: "1px solid rgba(255, 255, 255, 0.08)",
              borderRadius: "16px",
              padding: "24px",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "10px",
                marginBottom: "20px",
              }}
            >
              <div
                style={{
                  width: "32px",
                  height: "32px",
                  borderRadius: "8px",
                  background: "rgba(245, 158, 11, 0.15)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  color: "#fbbf24",
                }}
              >
                <Award size={18} />
              </div>
              <div>
                <h2 style={{ fontSize: "18px", fontWeight: 700, margin: 0 }}>
                  Top 5 Days Listened to Most
                </h2>
                <p
                  style={{
                    margin: 0,
                    fontSize: "12px",
                    color: "rgba(255, 255, 255, 0.4)",
                  }}
                >
                  Ranked by peak listening time in a single day
                </p>
              </div>
            </div>

            {!stats?.top_days || stats.top_days.length === 0 ? (
              <div
                style={{
                  padding: "36px 0",
                  textAlign: "center",
                  color: "rgba(255, 255, 255, 0.4)",
                  fontSize: "13px",
                }}
              >
                No daily listening peaks logged yet. Keep playing your favorite tunes!
              </div>
            ) : (
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
                  gap: "14px",
                }}
              >
                {stats.top_days.map((day, idx) => {
                  const rank = idx + 1;
                  const rankBadgeBg =
                    rank === 1
                      ? "linear-gradient(135deg, #fbbf24, #d97706)"
                      : rank === 2
                      ? "linear-gradient(135deg, #94a3b8, #64748b)"
                      : rank === 3
                      ? "linear-gradient(135deg, #d97706, #b45309)"
                      : "rgba(255, 255, 255, 0.1)";
                  return (
                    <div
                      key={day.day_date}
                      style={{
                        background: "rgba(255, 255, 255, 0.03)",
                        border: "1px solid rgba(255, 255, 255, 0.06)",
                        borderRadius: "12px",
                        padding: "16px",
                        position: "relative",
                        overflow: "hidden",
                      }}
                    >
                      <div
                        style={{
                          position: "absolute",
                          top: "12px",
                          right: "12px",
                          width: "24px",
                          height: "24px",
                          borderRadius: "50%",
                          background: rankBadgeBg,
                          color: "#fff",
                          fontSize: "12px",
                          fontWeight: 800,
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                        }}
                      >
                        #{rank}
                      </div>
                      <div
                        style={{
                          fontSize: "11px",
                          fontWeight: 700,
                          color: "#a78bfa",
                          textTransform: "uppercase",
                          letterSpacing: "0.5px",
                          marginBottom: "4px",
                        }}
                      >
                        {day.day_name}
                      </div>
                      <div
                        style={{
                          fontSize: "14px",
                          fontWeight: 600,
                          color: "#fff",
                          marginBottom: "8px",
                        }}
                      >
                        {day.day_date}
                      </div>
                      <div
                        style={{
                          fontSize: "20px",
                          fontWeight: 800,
                          color: "#34d399",
                        }}
                      >
                        {formatSeconds(day.total_seconds)}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </>
      ) : (
        /* Archived Year View (Previous Year Records) */
        <div>
          {/* Archived Year Hero Banner */}
          <div
            style={{
              background: "linear-gradient(135deg, #311042 0%, #18181b 100%)",
              border: "1px solid rgba(168, 85, 247, 0.3)",
              borderRadius: "20px",
              padding: "32px",
              marginBottom: "36px",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              flexWrap: "wrap",
              gap: "20px",
            }}
          >
            <div>
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  color: "#d8b4fe",
                  fontSize: "13px",
                  fontWeight: 700,
                  textTransform: "uppercase",
                  letterSpacing: "0.5px",
                  marginBottom: "6px",
                }}
              >
                <Archive size={16} />
                <span>Archived Year Record</span>
              </div>
              <h2
                style={{
                  fontSize: "36px",
                  fontWeight: 800,
                  margin: "0 0 8px 0",
                  letterSpacing: "-0.5px",
                }}
              >
                {viewingYear} in Review
              </h2>
              <p
                style={{
                  margin: 0,
                  color: "rgba(255, 255, 255, 0.6)",
                  fontSize: "14px",
                }}
              >
                Stored official archive of your top music listening data for the year {viewingYear}.
              </p>
            </div>

            <div
              style={{
                background: "rgba(0, 0, 0, 0.4)",
                border: "1px solid rgba(255, 255, 255, 0.1)",
                borderRadius: "16px",
                padding: "16px 24px",
                textAlign: "center",
              }}
            >
              <div
                style={{
                  fontSize: "12px",
                  color: "rgba(255, 255, 255, 0.5)",
                  fontWeight: 600,
                  textTransform: "uppercase",
                  marginBottom: "4px",
                }}
              >
                Total Listening Time
              </div>
              <div
                style={{
                  fontSize: "32px",
                  fontWeight: 800,
                  color: "#c084fc",
                }}
              >
                {formatSeconds(stats?.total_year_seconds || 0)}
              </div>
            </div>
          </div>

          {/* Archived Top 10 Songs & Top 10 Artists */}
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(480px, 1fr))",
              gap: "28px",
            }}
          >
            {/* Top 10 Songs of the Year */}
            <div
              style={{
                background: "#18181b",
                border: "1px solid rgba(255, 255, 255, 0.08)",
                borderRadius: "16px",
                padding: "24px",
              }}
            >
              <h3 style={{ fontSize: "18px", fontWeight: 700, margin: "0 0 16px 0" }}>
                Top 10 Songs of {viewingYear}
              </h3>
              {!stats?.top_songs || stats.top_songs.length === 0 ? (
                <div
                  style={{
                    padding: "36px 0",
                    textAlign: "center",
                    color: "rgba(255, 255, 255, 0.4)",
                    fontSize: "13px",
                  }}
                >
                  No songs recorded in the {viewingYear} archive.
                </div>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                  {stats.top_songs.map((song, idx) => (
                    <div
                      key={song.track_id}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        padding: "10px 14px",
                        borderRadius: "10px",
                        background: idx < 3 ? "rgba(255, 255, 255, 0.03)" : "transparent",
                      }}
                    >
                      <span
                        style={{
                          width: "28px",
                          fontSize: "15px",
                          fontWeight: 800,
                          color: idx === 0 ? "#fbbf24" : idx === 1 ? "#e2e8f0" : idx === 2 ? "#d97706" : "rgba(255, 255, 255, 0.5)",
                          textAlign: "center",
                        }}
                      >
                        {idx + 1}
                      </span>
                      <div style={{ flex: 1, minWidth: 0, margin: "0 14px" }}>
                        <div style={{ fontSize: "14px", fontWeight: 600, color: "#fff" }}>
                          {song.title}
                        </div>
                        <div style={{ fontSize: "12px", color: "rgba(255, 255, 255, 0.5)" }}>
                          {song.artist_name || "Unknown Artist"}
                        </div>
                      </div>
                      <div style={{ textAlign: "right" }}>
                        <div style={{ fontSize: "13px", fontWeight: 600, color: "#fff" }}>
                          {formatSeconds(song.total_seconds)}
                        </div>
                        <div style={{ fontSize: "11px", color: "rgba(255, 255, 255, 0.4)" }}>
                          {song.play_count} plays
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Top 10 Artists of the Year */}
            <div
              style={{
                background: "#18181b",
                border: "1px solid rgba(255, 255, 255, 0.08)",
                borderRadius: "16px",
                padding: "24px",
              }}
            >
              <h3 style={{ fontSize: "18px", fontWeight: 700, margin: "0 0 16px 0" }}>
                Top 10 Artists of {viewingYear}
              </h3>
              {!stats?.top_artists || stats.top_artists.length === 0 ? (
                <div
                  style={{
                    padding: "36px 0",
                    textAlign: "center",
                    color: "rgba(255, 255, 255, 0.4)",
                    fontSize: "13px",
                  }}
                >
                  No artists recorded in the {viewingYear} archive.
                </div>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                  {stats.top_artists.map((artist, idx) => (
                    <div
                      key={artist.artist_id}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        padding: "10px 14px",
                        borderRadius: "10px",
                        background: idx < 3 ? "rgba(255, 255, 255, 0.03)" : "transparent",
                      }}
                    >
                      <span
                        style={{
                          width: "28px",
                          fontSize: "15px",
                          fontWeight: 800,
                          color: idx === 0 ? "#fbbf24" : idx === 1 ? "#e2e8f0" : idx === 2 ? "#d97706" : "rgba(255, 255, 255, 0.5)",
                          textAlign: "center",
                        }}
                      >
                        {idx + 1}
                      </span>
                      <div style={{ flex: 1, minWidth: 0, margin: "0 14px" }}>
                        <div style={{ fontSize: "14px", fontWeight: 600, color: "#fff" }}>
                          {artist.name}
                        </div>
                      </div>
                      <div style={{ textAlign: "right" }}>
                        <div style={{ fontSize: "13px", fontWeight: 600, color: "#fff" }}>
                          {formatSeconds(artist.total_seconds)}
                        </div>
                        <div style={{ fontSize: "11px", color: "rgba(255, 255, 255, 0.4)" }}>
                          {artist.play_count} plays
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
