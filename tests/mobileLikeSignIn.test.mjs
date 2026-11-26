import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { build } from "esbuild";

const bundle = await build({
  entryPoints: ["src/presentation/mobile/mobileLikeActions.ts"],
  bundle: true,
  write: false,
  platform: "node",
  format: "esm",
  plugins: [{
    name: "capture-toast",
    setup(build) {
      build.onResolve({ filter: /^\.\/phoneToasts$/ }, ({ path }) => ({ path, namespace: "toast" }));
      build.onLoad({ filter: /.*/, namespace: "toast" }, () => ({
        contents: "export const showPhoneToast = (title) => globalThis.mobileLikeToasts.push(title);",
      }));
    },
  }],
});
const { mobileLikeActions } = await import(`data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString("base64")}`);

test("every Android like action prompts guests and skips the write", async () => {
  const notCalled = () => assert.fail("guest like was written");
  globalThis.mobileLikeToasts = [];
  try {
    const { c, player } = mobileLikeActions(
      { currentUser: null, handleLike: notCalled, handleToggleCollectionLike: notCalled },
      { onLike: notCalled, onLikeOnline: notCalled },
    );
    await c.handleLike("download");
    await c.handleToggleCollectionLike({ id: "collection-track" }, false);
    player.onLike("download");
    player.onLikeOnline({ id: "online:track" });
    assert.deepEqual(globalThis.mobileLikeToasts, Array(4).fill("Sign in to like songs"));
  } finally {
    delete globalThis.mobileLikeToasts;
  }
});

test("signed-in Android users keep the shared like handlers", () => {
  const c = { currentUser: { id: "signed-in" }, handleLike: () => {} };
  const player = { onLike: () => {} };
  const actions = mobileLikeActions(c, player);
  assert.equal(actions.c, c);
  assert.equal(actions.player, player);
});

test("the mobile shell passes guarded actions to its screens and player", () => {
  const app = readFileSync(new URL("../src/presentation/mobile/MobileApp.tsx", import.meta.url), "utf8");
  assert.match(app, /const \{ c, player \} = useMemo\(\(\) => mobileLikeActions\(viewContext, playback\)/);
  assert.match(app, /<TrackSheets c=\{c\}/);
  assert.match(app, /<MobileDeck c=\{c\} player=\{player\}/);
  assert.match(app, /<MobileLyrics player=\{player\}/);
});
