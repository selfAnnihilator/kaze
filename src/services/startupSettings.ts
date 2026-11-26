import type { AppSettings, Query, QueryResponse } from "../types";

type SettingsQuery = (query: Query) => Promise<QueryResponse>;
const MOBILE_RETRY_DELAYS_MS = [250, 500, 1000];

async function requestSettings(query: SettingsQuery): Promise<AppSettings> {
  const response = await query({ query: "GetSettings" });
  if (response.type !== "Settings" || !response.data || typeof response.data !== "object") {
    throw new Error("Startup settings are unavailable");
  }
  return response.data as AppSettings;
}

/** Keep the phone's opening screen through a transient bridge/settings failure. */
export async function readStartupSettings(query: SettingsQuery, mobile = false): Promise<AppSettings> {
  const delays = mobile ? MOBILE_RETRY_DELAYS_MS : [];
  for (let attempt = 0; ; attempt++) {
    try {
      return await requestSettings(query);
    } catch (error) {
      const delay = delays[attempt];
      if (delay === undefined) throw error;
      await new Promise<void>((resolve) => setTimeout(resolve, delay));
    }
  }
}
