import type { FlowExplanation } from "./types";

export interface ExplanationCopy { title: string; body: string; secondary: string[] }

// Never infer statistics from legacy prose. Selection evidence remains a snapshot.
export function presentFlowExplanation(e?: FlowExplanation | null): ExplanationCopy {
  const days = e?.days_since_played;
  const age = days != null && days >= 0
    ? days >= 90 ? `about ${Math.floor(days / 30)} months` : `${days} ${days === 1 ? "day" : "days"}`
    : null;
  const snapshot = e?.selected_at != null;
  const unplayed = snapshot ? "You hadn't played this track in Kaze when it was picked." : "You haven't played this track in Kaze yet.";
  const rediscovery = age ? `It's been ${age} since your last recorded play${snapshot ? " at the time of this pick" : ""}.` : "A track from your library to revisit.";
  const affinity = e?.artist_affinity != null && e.artist_affinity > 0.3 && !!e.artist_name;
  let title = "From your library";
  let body = "A pick from your own music collection.";
  if (e?.from_existing_queue) { title = "Your queue, your choice"; body = "You already had this track in your queue. Flow kept it in place."; }
  else if (e?.primary === "continuing_track") { title = "Your current track continues"; body = "Flow will take over with the next track. This song can finish playing."; }
  else if (e?.primary === "liked" && e.is_liked) { title = "One of your favorites"; body = "You liked this track, so it's a familiar choice to return to."; }
  else if (e?.primary === "rediscovery" && age) { title = "Time to rediscover this"; body = rediscovery; }
  else if (e?.primary === "library_exploration" && e.is_unplayed) { title = "A fresh pick from your library"; body = unplayed; }
  else if (e?.primary === "artist_affinity" && affinity) { title = `More from ${e.artist_name}`; body = `Your listening and feedback show an affinity for ${e.artist_name}.`; }
  const secondary: string[] = [];
  if (!e?.from_existing_queue) {
    if (e?.is_liked && e.primary !== "liked") secondary.push("You liked this track.");
    if (age && days! >= 30 && e?.primary !== "rediscovery") secondary.push(rediscovery);
    if (e?.is_unplayed && e.primary !== "library_exploration") secondary.push(unplayed);
    if (affinity && e?.primary !== "artist_affinity") secondary.push(`Your listening and feedback point toward ${e?.artist_name}.`);
    if (e?.play_count != null && e.play_count > 0 && secondary.length < 2) secondary.push(`${e.play_count} recorded ${e.play_count === 1 ? "play" : "plays"} in Kaze${snapshot ? " when this was picked" : ""}.`);
    if (e?.familiarity_ratio != null && secondary.length < 2) secondary.push(e.familiarity_ratio >= .75 ? "Picked with Flow leaning toward familiar music." : e.familiarity_ratio <= .25 ? "Picked with Flow leaning toward rediscovery." : "Picked with Flow balancing familiar music and rediscovery.");
  }
  return { title, body, secondary: secondary.slice(0, 2) };
}
