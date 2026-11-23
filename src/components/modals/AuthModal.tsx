import React, { useState, useEffect } from "react";
import { X, Lock, User, AlertCircle, ArrowRight } from "lucide-react";
import { UserProfile } from "../../types";

interface AuthModalProps {
  isOpen: boolean;
  onClose: () => void;
  onLogin: (username: string, password: string) => Promise<UserProfile | null>;
  onSignUp: (username: string, password: string) => Promise<UserProfile | null>;
}

export const AuthModal: React.FC<AuthModalProps> = ({
  isOpen,
  onClose,
  onLogin,
  onSignUp,
}) => {
  const [tab, setTab] = useState<"login" | "signup">("login");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const clearFields = () => {
    setUsername("");
    setPassword("");
    setConfirmPassword("");
    setError(null);
  };

  // Explicitly clear all credentials whenever modal opens or switches tabs
  useEffect(() => {
    if (isOpen) {
      clearFields();
    }
  }, [isOpen, tab]);

  if (!isOpen) return null;

  const handleClose = () => {
    clearFields();
    onClose();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    const trimmedUser = username.trim();
    if (!trimmedUser) {
      setError("Please enter a username.");
      return;
    }

    if (!password || password.length < 3) {
      setError("Password must be at least 3 characters.");
      return;
    }

    if (tab === "signup" && password !== confirmPassword) {
      setError("Passwords do not match.");
      return;
    }

    setIsLoading(true);
    try {
      if (tab === "login") {
        const profile = await onLogin(trimmedUser, password);
        if (profile) {
          handleClose();
        }
      } else {
        const profile = await onSignUp(trimmedUser, password);
        if (profile) {
          handleClose();
        }
      }
    } catch (err: any) {
      const msg = typeof err === "string" ? err : err?.message || "Authentication failed. Please check your credentials.";
      setError(msg);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div
      className="modal-overlay-anim"
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "var(--shade-75)",
        backdropFilter: "blur(6px)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: "var(--z-dialog)",
      }}
      onClick={handleClose}
    >
      <div
        className="modal-card-anim"
        style={{
          width: "100%",
          maxWidth: "400px",
          backgroundColor: "var(--palette-gray-900)",
          border: "1px solid var(--overlay-10)",
          borderRadius: "var(--radius-16)",
          padding: "var(--space-24)",
          boxShadow: "0 25px 50px -12px var(--shade-70)",
          color: "var(--text-main)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "var(--space-20)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-10)" }}>
            <div
              style={{
                width: "36px",
                height: "36px",
                borderRadius: "var(--radius-10)",
                background: "linear-gradient(135deg, var(--accent), var(--palette-taupe-550))",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <img src="/kaze-icon.png" alt="Kaze" style={{ width: "36px", height: "36px", borderRadius: "var(--radius-10)" }} />
            </div>
            <div>
              <h2 style={{ margin: 0, fontSize: "var(--fs-18)", fontWeight: "var(--weight-bold)" }}>
                {tab === "login" ? "Welcome Back" : "Create Account"}
              </h2>
              <p
                style={{
                  margin: 0,
                  fontSize: "var(--fs-12)",
                  color: "var(--overlay-50)",
                }}
              >
                Store your playlists and listening stats
              </p>
            </div>
          </div>
          <button
            onClick={handleClose}
            style={{
              background: "transparent",
              border: "none",
              color: "var(--overlay-50)",
              cursor: "pointer",
              padding: "var(--space-4)",
              borderRadius: "var(--radius-6)",
            }}
          >
            <X size={20} />
          </button>
        </div>

        {/* Tab Switcher */}
        <div
          style={{
            display: "flex",
            background: "var(--overlay-5)",
            borderRadius: "var(--radius-10)",
            padding: "var(--space-4)",
            marginBottom: "var(--space-18)",
          }}
        >
          <button
            type="button"
            onClick={() => {
              setTab("login");
              setError(null);
            }}
            style={{
              flex: 1,
              padding: "var(--space-8)",
              background: tab === "login" ? "var(--palette-gray-850)" : "transparent",
              color: tab === "login" ? "var(--text-main)" : "var(--overlay-50)",
              border: "none",
              borderRadius: "var(--radius-8)",
              fontSize: "var(--fs-13)",
              fontWeight: "var(--weight-semibold)",
              cursor: "pointer",
              transition: "transform var(--duration-fast) ease, opacity var(--duration-fast) ease",
            }}
          >
            Sign In
          </button>
          <button
            type="button"
            onClick={() => {
              setTab("signup");
              setError(null);
            }}
            style={{
              flex: 1,
              padding: "var(--space-8)",
              background: tab === "signup" ? "var(--palette-gray-850)" : "transparent",
              color: tab === "signup" ? "var(--text-main)" : "var(--overlay-50)",
              border: "none",
              borderRadius: "var(--radius-8)",
              fontSize: "var(--fs-13)",
              fontWeight: "var(--weight-semibold)",
              cursor: "pointer",
              transition: "transform var(--duration-fast) ease, opacity var(--duration-fast) ease",
            }}
          >
            Sign Up
          </button>
        </div>

        {/* Error Alert */}
        {error && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "var(--space-8)",
              background: "var(--danger-a15)",
              border: "1px solid var(--danger-a30)",
              color: "var(--palette-red-300)",
              padding: "var(--space-10) var(--space-12)",
              borderRadius: "var(--radius-8)",
              fontSize: "var(--fs-12)",
              marginBottom: "var(--space-16)",
            }}
          >
            <AlertCircle size={16} style={{ flexShrink: 0 }} />
            <span>{error}</span>
          </div>
        )}

        {/* Form */}
        <form onSubmit={handleSubmit} autoComplete="off" style={{ display: "flex", flexDirection: "column", gap: "var(--space-14)" }}>
          <div>
            <label
              style={{
                display: "block",
                fontSize: "var(--fs-12)",
                fontWeight: "var(--weight-semibold)",
                color: "var(--overlay-70)",
                marginBottom: "var(--space-6)",
              }}
            >
              Username
            </label>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "var(--space-10)",
                background: "var(--palette-gray-850)",
                borderRadius: "var(--radius-10)",
                padding: "0 var(--space-12)",
                border: "1px solid var(--overlay-10)",
              }}
            >
              <User size={16} color="var(--overlay-40)" />
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="e.g. musiclover"
                required
                autoFocus
                autoComplete="off"
                style={{
                  flex: 1,
                  background: "transparent",
                  border: "none",
                  padding: "var(--space-10) 0",
                  color: "var(--text-main)",
                  fontSize: "var(--fs-14)",
                  outline: "none",
                }}
              />
            </div>
          </div>

          <div>
            <label
              style={{
                display: "block",
                fontSize: "var(--fs-12)",
                fontWeight: "var(--weight-semibold)",
                color: "var(--overlay-70)",
                marginBottom: "var(--space-6)",
              }}
            >
              Password
            </label>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "var(--space-10)",
                background: "var(--palette-gray-850)",
                borderRadius: "var(--radius-10)",
                padding: "0 var(--space-12)",
                border: "1px solid var(--overlay-10)",
              }}
            >
              <Lock size={16} color="var(--overlay-40)" />
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Enter password"
                required
                autoComplete="new-password"
                style={{
                  flex: 1,
                  background: "transparent",
                  border: "none",
                  padding: "var(--space-10) 0",
                  color: "var(--text-main)",
                  fontSize: "var(--fs-14)",
                  outline: "none",
                }}
              />
            </div>
          </div>

          {tab === "signup" && (
            <div>
              <label
                style={{
                  display: "block",
                  fontSize: "var(--fs-12)",
                  fontWeight: "var(--weight-semibold)",
                  color: "var(--overlay-70)",
                  marginBottom: "var(--space-6)",
                }}
              >
                Confirm Password
              </label>
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "var(--space-10)",
                  background: "var(--palette-gray-850)",
                  borderRadius: "var(--radius-10)",
                  padding: "0 var(--space-12)",
                  border: "1px solid var(--overlay-10)",
                }}
              >
                <Lock size={16} color="var(--overlay-40)" />
                <input
                  type="password"
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  placeholder="Re-enter password"
                  required
                  autoComplete="new-password"
                  style={{
                    flex: 1,
                    background: "transparent",
                    border: "none",
                    padding: "var(--space-10) 0",
                    color: "var(--text-main)",
                    fontSize: "var(--fs-14)",
                    outline: "none",
                  }}
                />
              </div>
            </div>
          )}

          <button
            type="submit"
            disabled={isLoading}
            style={{
              marginTop: "var(--space-8)",
              padding: "var(--space-12)",
              background: "linear-gradient(135deg, var(--accent), var(--palette-taupe-550))",
              color: "var(--text-main)",
              border: "none",
              borderRadius: "var(--radius-10)",
              fontSize: "var(--fs-14)",
              fontWeight: "var(--weight-semibold)",
              cursor: isLoading ? "not-allowed" : "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: "var(--space-8)",
              opacity: isLoading ? 0.7 : 1,
              boxShadow: "0 4px 12px var(--accent-a30)",
              transition: "transform var(--duration-instant) ease",
            }}
          >
            <span>{tab === "login" ? "Sign In" : "Create Account"}</span>
            <ArrowRight size={16} />
          </button>
        </form>
      </div>
    </div>
  );
};
