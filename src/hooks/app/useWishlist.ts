import { useCallback, useMemo, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import { dispatchCommand, executeQuery } from "../../services/api";
import type { WishlistItem } from "../../types";

/** Wishlist: account-scoped fetch, add, add-missing, and status updates. */
export function useWishlist({ libraryAccountRef }: { libraryAccountRef: MutableRefObject<string> }) {
  const [wishlist, setWishlist] = useState<WishlistItem[]>([]);

  const wishlistRequest = useRef(0);
  const fetchWishlist = useCallback(async () => {
    const account = libraryAccountRef.current, request = ++wishlistRequest.current;
    try {
      const res = await executeQuery({ query: "GetWishlist" });
      if (account === libraryAccountRef.current && request === wishlistRequest.current && Array.isArray(res.data)) {
        setWishlist(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch wishlist:", err);
    }
  }, []);

  const handleAddMissingToWishlist = useCallback(async (missingTracks: any[]) => {
    await dispatchCommand({
      command: "AddMissingToWishlist",
      payload: { tracks: missingTracks },
    });
    fetchWishlist();
  }, [fetchWishlist]);

  const handleAddToWishlist = useCallback(async (
    title: string,
    artist: string,
    album?: string,
  ) => {
    await dispatchCommand({
      command: "AddToWishlist",
      payload: { title, artist, album },
    });
    fetchWishlist();
  }, [fetchWishlist]);

  const handleUpdateWishlistStatus = useCallback(async (
    wishlist_id: string,
    status: "want" | "ignore" | "already_own" | "downloaded",
  ) => {
    await dispatchCommand({
      command: "UpdateWishlistStatus",
      payload: { wishlist_id, status },
    });
    await fetchWishlist();
  }, [fetchWishlist]);

  return useMemo(() => ({
    wishlist, setWishlist, fetchWishlist, handleAddMissingToWishlist, handleAddToWishlist, handleUpdateWishlistStatus,
  }), [wishlist, setWishlist, fetchWishlist, handleAddMissingToWishlist, handleAddToWishlist, handleUpdateWishlistStatus]);
}
