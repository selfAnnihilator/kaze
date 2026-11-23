import { showPhoneToast } from "./phoneToasts";

/**
 * The banner's UPDATE button (Figma G10). In-app updating on Android is not
 * built yet; this is the one place to start it once it is.
 */
export function startAppUpdate(version: string) {
  showPhoneToast(`Updating to v${version} from the app is coming soon`);
}
