import { useCallback, useMemo, useRef, useState } from "react";
import { dispatchCommand, executeQuery } from "../../services/api";
import type { OnboardingStatus, UserProfile } from "../../types";

/** Called after a successful login/sign-up: toast + account-scoped refetches. */
export type AfterAuth = (title: string, message: string) => Promise<void>;

/**
 * Session identity and onboarding: current user (+ hydration), account refs
 * used to guard stale queries, login/sign-up/logout and onboarding status.
 * App wires post-auth side effects through `afterAuthRef` (assigned each render).
 */
export function useAuth({ resetNavigation }: { resetNavigation: () => void }) {
  // Onboarding
  const [onboardingStatus, setOnboardingStatus] = useState<OnboardingStatus | null>(null);
  const [showOnboarding, setShowOnboarding] = useState(false);

  // User Authentication & Profile
  const [currentUser, setCurrentUser] = useState<UserProfile | null>(null);
  const [userHydrated, setUserHydrated] = useState(false);
  const currentUserIdRef = useRef("default");
  const [isAuthModalOpen, setIsAuthModalOpen] = useState(false);
  const libraryAccountRef = useRef(currentUser?.id || "default");
  libraryAccountRef.current = currentUser?.id || "default";
  const afterAuthRef = useRef<AfterAuth | null>(null);

  const fetchOnboardingStatus = useCallback(async () => {
    try {
      const res = await executeQuery({ query: "GetOnboardingStatus" });
      const status: OnboardingStatus = {
        completed: res.data?.completed ?? res.completed ?? false,
        default_music_dir: res.data?.default_music_dir ?? res.default_music_dir ?? "",
        configured_folders: res.data?.configured_folders ?? res.configured_folders ?? [],
      };
      setOnboardingStatus(status);
      if (!status.completed) {
        setShowOnboarding(true);
      }
    } catch (err) {
      console.error("Failed to check onboarding status:", err);
    }
  }, []);

  const userQueryRevision = useRef(0);
  const fetchCurrentUser = useCallback(async () => {
    const revision = ++userQueryRevision.current;
    try {
      const res = await executeQuery({ query: "GetCurrentUser" });
      if (revision !== userQueryRevision.current) return;
      const nextAccount = res.data?.id || "default";
      if (nextAccount !== libraryAccountRef.current) resetNavigation();
      if (res && res.data) {
        setCurrentUser(res.data);
      } else {
        setCurrentUser(null);
      }
    } catch (err) {
      console.error("Failed to fetch current user:", err);
    } finally { setUserHydrated(true); }
  }, []);

  const handleLogin = useCallback(async (
    username: string,
    password: string,
  ): Promise<UserProfile | null> => {
    const res = await dispatchCommand({
      command: "Login",
      payload: { username, password },
    });
    if (res && res.status === "UserProfile" && res.data) {
      ++userQueryRevision.current;
      libraryAccountRef.current = res.data.id;
      setCurrentUser(res.data);
      await afterAuthRef.current?.("Logged In", `Welcome back, ${res.data.username}!`);
      return res.data;
    }
    return null;
  }, []);

  const handleSignUp = useCallback(async (
    username: string,
    password: string,
  ): Promise<UserProfile | null> => {
    const res = await dispatchCommand({
      command: "SignUp",
      payload: { username, password },
    });
    if (res && res.status === "UserProfile" && res.data) {
      ++userQueryRevision.current;
      libraryAccountRef.current = res.data.id;
      setCurrentUser(res.data);
      await afterAuthRef.current?.("Account Created", `Welcome to Kaze, ${res.data.username}!`);
      return res.data;
    }
    return null;
  }, []);

  const handleLogout = useCallback(async (scope: "device" | "all") => {
    await dispatchCommand({ command: scope === "all" ? "LogoutAll" : "Logout" });
    window.location.reload();
  }, []);

  return useMemo(() => ({
    onboardingStatus, showOnboarding, setShowOnboarding,
    currentUser, setCurrentUser, userHydrated, currentUserIdRef, libraryAccountRef, userQueryRevision,
    isAuthModalOpen, setIsAuthModalOpen, afterAuthRef,
    fetchOnboardingStatus, fetchCurrentUser, handleLogin, handleSignUp, handleLogout,
  }), [onboardingStatus, showOnboarding, currentUser, userHydrated, isAuthModalOpen,
    fetchOnboardingStatus, fetchCurrentUser, handleLogin, handleSignUp, handleLogout]);
}
