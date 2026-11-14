import { Music, WifiOff } from "lucide-react";

/** Discover without a connection: points to the local library instead. */
export function OfflineDiscovery({ onGoToLibrary }: { onGoToLibrary?: () => void }) {
  return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          minHeight: "65vh",
          textAlign: "center",
          padding: "var(--space-40) var(--space-20)",
        }}
      >
        <div
          style={{
            backgroundColor: "var(--palette-gray-900-a95)",
            border: "1px solid var(--danger-a35)",
            borderRadius: "var(--radius-16)",
            padding: "var(--space-48) var(--space-36)",
            textAlign: "center",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: "var(--space-12)",
            maxWidth: "460px",
            width: "100%",
            boxShadow: "0 8px 30px var(--shade-40)",
          }}
        >
          <div
            style={{
              width: "56px",
              height: "56px",
              borderRadius: "50%",
              backgroundColor: "var(--danger-a12)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              marginBottom: "var(--space-4)",
            }}
          >
            <WifiOff size={28} color="var(--danger)" />
          </div>
          <h2
            style={{
              fontSize: "var(--fs-20)",
              fontWeight: "var(--weight-bold)",
              color: "var(--text-main)",
              margin: 0,
            }}
          >
            No internet connection
          </h2>
          <p
            style={{
              fontSize: "var(--fs-15)",
              color: "var(--text-muted)",
              margin: 0,
              maxWidth: "420px",
              lineHeight: 1.5,
            }}
          >
            No internet connection. Listen to local songs.
          </p>
          {onGoToLibrary && (
            <button
              type="button"
              className="btn btn-primary"
              onClick={onGoToLibrary}
              style={{
                marginTop: "var(--space-12)",
                padding: "var(--space-10) var(--space-20)",
                fontSize: "var(--fs-14)",
                borderRadius: "var(--radius-10)",
                display: "inline-flex",
                alignItems: "center",
                gap: "var(--space-8)",
                fontWeight: "var(--weight-semibold)",
              }}
            >
              <Music size={16} />
              <span>Go to Downloaded Songs</span>
            </button>
          )}
        </div>
      </div>
  );
}
