import React, { useState, useEffect, useCallback, useRef } from "react";
import { DailyListeningPoint, StatsOverview, UserProfile } from "../../types";
import { dispatchCommand } from "../../services/api";
import type { StatsViewContext } from "./stats/context";
import { ProfileCard } from "./stats/ProfileCard";
import { StatsHeader } from "./stats/StatsHeader";
import { DailyWeeklyStats } from "./stats/DailyWeeklyStats";
import { MonthlyStats } from "./stats/MonthlyStats";
import { TopSongsArtists } from "./stats/TopSongsArtists";
import { TopDays } from "./stats/TopDays";
import { ArchivedYearHero } from "./stats/ArchivedYearHero";
import { ArchivedTopLists } from "./stats/ArchivedTopLists";

import { MONTH_NAMES } from "./stats/monthNames";

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
      const res = await dispatchCommand({ command: "UploadAvatar", payload: {} });
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

  const c: StatsViewContext = {
    avatarContainerRef, currentUser, onOpenAuthModal, setShowAvatarMenu, handlePickAvatar,
    isUploadingAvatar, showAvatarMenu, handleRemoveAvatar, formatDate, avatarError, viewingYear, stats,
    selectedYear, currentYear, setSelectedYear, loadStats, isLoading, isRefreshing, formatSeconds,
    activeMonthName, activeMonthNum, setSelectedMonth, hoveredDay, monthlyGraph, daysWithMusic,
    dailyAvgSecs, maxGraphSeconds, setHoveredDay, onPlayTrack, isPastYear,
  };
  return (
    <div
      className="stats-view"
      style={{
        padding: "var(--space-32) var(--space-40) calc(100 * var(--space-unit))",
        maxWidth: "1200px",
        margin: "0 auto",
        color: "var(--text-main)",
      }}
    >
      {/* Profile Section Header Card */}
      <ProfileCard c={c} />

      {/* Listening Stats Header Banner */}
      <StatsHeader c={c} />

      {isLoading ? (
        <div
          style={{
            padding: "var(--space-80) 0",
            textAlign: "center",
            color: "var(--overlay-50)",
          }}
        >
          <div
            className="spinner"
            style={{
              width: "36px",
              height: "36px",
              border: "3px solid var(--accent-a20)",
              borderTopColor: "var(--accent)",
              borderRadius: "50%",
              margin: "0 auto var(--space-16)",
              animation: "spin 1s linear infinite",
            }}
          />
          <p>Loading your listening records...</p>
        </div>
      ) : !isPastYear ? (
        <>
          {/* Row 1: Daily and Weekly Section (2-Column Grid) */}
          <DailyWeeklyStats c={c} />

          {/* Row 2: Monthly Stats Section (Takes Whole Width Below Daily & Weekly) */}
          <MonthlyStats c={c} />

          {/* Grid Layout: Top 10 Songs & Top 5 Artists */}
          <TopSongsArtists c={c} />

          {/* Top 5 Days User Listened to Most */}
          <TopDays c={c} />
        </>
      ) : (
        /* Archived Year View (Previous Year Records) */
        <div>
          {/* Archived Year Hero Banner */}
          <ArchivedYearHero c={c} />

          {/* Archived Top 10 Songs & Top 5 Artists */}
          <ArchivedTopLists c={c} />
        </div>
      )}
    </div>
  );
};
