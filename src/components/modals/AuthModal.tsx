import React, { useState, useEffect } from "react";
import { X, Lock, User, Sparkles, AlertCircle, ArrowRight } from "lucide-react";
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
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "rgba(0, 0, 0, 0.75)",
        backdropFilter: "blur(6px)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 9999,
      }}
      onClick={handleClose}
    >
      <div
        style={{
          width: "100%",
          maxWidth: "400px",
          backgroundColor: "#18181b",
          border: "1px solid rgba(255, 255, 255, 0.1)",
          borderRadius: "16px",
          padding: "24px",
          boxShadow: "0 25px 50px -12px rgba(0, 0, 0, 0.7)",
          color: "#fff",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "20px",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <div
              style={{
                width: "36px",
                height: "36px",
                borderRadius: "10px",
                background: "linear-gradient(135deg, #8b5cf6, #6366f1)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <Sparkles size={20} color="#fff" />
            </div>
            <div>
              <h2 style={{ margin: 0, fontSize: "18px", fontWeight: 700 }}>
                {tab === "login" ? "Welcome Back" : "Create Account"}
              </h2>
              <p
                style={{
                  margin: 0,
                  fontSize: "12px",
                  color: "rgba(255, 255, 255, 0.5)",
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
              color: "rgba(255, 255, 255, 0.5)",
              cursor: "pointer",
              padding: "4px",
              borderRadius: "6px",
            }}
          >
            <X size={20} />
          </button>
        </div>

        {/* Tab Switcher */}
        <div
          style={{
            display: "flex",
            background: "rgba(255, 255, 255, 0.05)",
            borderRadius: "10px",
            padding: "3px",
            marginBottom: "18px",
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
              padding: "8px",
              background: tab === "login" ? "#27272a" : "transparent",
              color: tab === "login" ? "#fff" : "rgba(255, 255, 255, 0.5)",
              border: "none",
              borderRadius: "8px",
              fontSize: "13px",
              fontWeight: 600,
              cursor: "pointer",
              transition: "all 0.15s ease",
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
              padding: "8px",
              background: tab === "signup" ? "#27272a" : "transparent",
              color: tab === "signup" ? "#fff" : "rgba(255, 255, 255, 0.5)",
              border: "none",
              borderRadius: "8px",
              fontSize: "13px",
              fontWeight: 600,
              cursor: "pointer",
              transition: "all 0.15s ease",
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
              gap: "8px",
              background: "rgba(239, 68, 68, 0.15)",
              border: "1px solid rgba(239, 68, 68, 0.3)",
              color: "#f87171",
              padding: "10px 12px",
              borderRadius: "8px",
              fontSize: "12px",
              marginBottom: "16px",
            }}
          >
            <AlertCircle size={16} style={{ flexShrink: 0 }} />
            <span>{error}</span>
          </div>
        )}

        {/* Form */}
        <form onSubmit={handleSubmit} autoComplete="off" style={{ display: "flex", flexDirection: "column", gap: "14px" }}>
          <div>
            <label
              style={{
                display: "block",
                fontSize: "12px",
                fontWeight: 600,
                color: "rgba(255, 255, 255, 0.7)",
                marginBottom: "6px",
              }}
            >
              Username
            </label>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "10px",
                background: "#27272a",
                borderRadius: "10px",
                padding: "0 12px",
                border: "1px solid rgba(255, 255, 255, 0.1)",
              }}
            >
              <User size={16} color="rgba(255, 255, 255, 0.4)" />
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
                  padding: "10px 0",
                  color: "#fff",
                  fontSize: "14px",
                  outline: "none",
                }}
              />
            </div>
          </div>

          <div>
            <label
              style={{
                display: "block",
                fontSize: "12px",
                fontWeight: 600,
                color: "rgba(255, 255, 255, 0.7)",
                marginBottom: "6px",
              }}
            >
              Password
            </label>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "10px",
                background: "#27272a",
                borderRadius: "10px",
                padding: "0 12px",
                border: "1px solid rgba(255, 255, 255, 0.1)",
              }}
            >
              <Lock size={16} color="rgba(255, 255, 255, 0.4)" />
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
                  padding: "10px 0",
                  color: "#fff",
                  fontSize: "14px",
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
                  fontSize: "12px",
                  fontWeight: 600,
                  color: "rgba(255, 255, 255, 0.7)",
                  marginBottom: "6px",
                }}
              >
                Confirm Password
              </label>
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "10px",
                  background: "#27272a",
                  borderRadius: "10px",
                  padding: "0 12px",
                  border: "1px solid rgba(255, 255, 255, 0.1)",
                }}
              >
                <Lock size={16} color="rgba(255, 255, 255, 0.4)" />
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
                    padding: "10px 0",
                    color: "#fff",
                    fontSize: "14px",
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
              marginTop: "8px",
              padding: "11px",
              background: "linear-gradient(135deg, #8b5cf6, #6366f1)",
              color: "#fff",
              border: "none",
              borderRadius: "10px",
              fontSize: "14px",
              fontWeight: 600,
              cursor: isLoading ? "not-allowed" : "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: "8px",
              opacity: isLoading ? 0.7 : 1,
              boxShadow: "0 4px 12px rgba(139, 92, 246, 0.3)",
              transition: "transform 0.1s ease",
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
