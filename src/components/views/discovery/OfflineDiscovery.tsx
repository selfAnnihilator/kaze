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
          padding: "40px 20px",
        }}
      >
        <div
          style={{
            backgroundColor: "var(--palette-gray-900-a95)",
            border: "1px solid var(--danger-a35)",
            borderRadius: "16px",
            padding: "48px 36px",
            textAlign: "center",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: "12px",
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
              marginBottom: "4px",
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
                marginTop: "12px",
                padding: "9px 20px",
                fontSize: "var(--fs-14)",
                borderRadius: "10px",
                display: "inline-flex",
                alignItems: "center",
                gap: "8px",
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
