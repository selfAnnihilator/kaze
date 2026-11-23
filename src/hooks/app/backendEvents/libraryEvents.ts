// Library scan, download, wishlist and playlist events.
import { clearCoverArtCache } from "../../../services/coverArt";
import { downloadProgress } from "../../../services/downloadProgress";
import { startDownload } from "../../../services/downloadActions";

/** RETRY on a failed download's toast, when the song behind the task is known. */
function retryAction(taskId: string | undefined) {
  const trackId = taskId ? downloadProgress.trackOfTask(taskId) : undefined;
  return trackId ? { label: "RETRY", run: () => void startDownload(trackId).catch(() => undefined) } : undefined;
}
import type { BackendEvent, BackendEventContext } from "./context";
import { cloudSyncSignal } from "../../../services/cloudSyncSignal";

/** Handles one library event; returns false when the event belongs elsewhere. */
export function handleLibraryEvent(event: BackendEvent, ctx: BackendEventContext): boolean {
  const { addAppNotification, fetchAlbums, fetchDiscovery, fetchDownloads, fetchOnboardingStatus, fetchPlaybackState, fetchPlaylists, fetchTrackPlaylistMemberships, fetchTracks, fetchWishlist, setIsScanning } = ctx;
  switch (event.event) {
    case "LibraryScanStarted":
      setIsScanning(true);
      break;

    case "LibraryMetadataUpdated":
      clearCoverArtCache();
      fetchTracks();
      fetchAlbums();
      fetchPlaybackState();
      break;
    case "LibraryScanCompleted":
      clearCoverArtCache();
      setIsScanning(false);
      fetchDownloads();
      addAppNotification(
        "info",
        "Library Scan Complete",
        "Your local music library has been updated with the latest tracks.",
      );
      fetchTracks();
      fetchAlbums();
      fetchOnboardingStatus();
      fetchDiscovery();
      break;

    case "LibraryScanFailed":
      setIsScanning(false);
      addAppNotification(
        "error",
        "Library Scan Failed",
        "An error occurred while scanning your music folders.",
      );
      break;

    case "DownloadQueued":
      fetchDownloads();
      break;

    // Files in Kaze's folder were added, removed or renamed (watched live).
    case "DownloadsChanged":
      fetchDownloads();
      fetchTracks();
      // A removed download's likes and playlist entries move to its online song.
      fetchTrackPlaylistMemberships();
      break;

    case "DownloadProgress":
    case "DownloadProgressChanged":
      downloadProgress.update(event.payload?.task_id, event.payload?.bytes_downloaded,
        event.payload?.total_bytes || event.payload?.file_size);
      break;

    case "DownloadCompleted": {
      const trackTitle = event.payload?.title || "Track";
      const artist = event.payload?.artist
        ? ` by ${event.payload.artist}`
        : "";
      addAppNotification(
        "success",
        "Download Complete",
        `"${trackTitle}"${artist} has been downloaded and added to your library.`,
      );
      fetchDownloads();
      fetchWishlist();
      fetchTracks();
      fetchAlbums();
      fetchDiscovery();
      break;
    }

    case "DownloadFailed": {
      const trackTitle = event.payload?.title || "Track";
      const errReason =
        event.payload?.error ||
        event.payload?.message ||
        "Connection timed out or peer unavailable.";
      addAppNotification(
        "error",
        "Download Failed",
        `Failed downloading "${trackTitle}": ${errReason}`,
        retryAction(event.payload?.task_id),
      );
      fetchDownloads();
      fetchWishlist();
      fetchTracks();
      fetchAlbums();
      fetchDiscovery();
      break;
    }

    case "WishlistUpdated":
      fetchWishlist();
      break;

    case "CloudDataSynced": {
      cloudSyncSignal.emit();
      break;
    }
    case "PlaylistsUpdated": {
      fetchPlaylists();
      fetchTrackPlaylistMemberships();
      fetchTracks();
      break;
    }

    default:
      return false;
  }
  return true;
}
