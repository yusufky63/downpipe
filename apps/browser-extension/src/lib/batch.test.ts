import { describe, expect, it } from "vitest";
import { runLimited } from "./batch";

const tick = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

describe("runLimited", () => {
  it("runs every item, never more than the limit at once, starting in order", async () => {
    let running = 0;
    let peak = 0;
    const started: number[] = [];
    const count = await runLimited([1, 2, 3, 4, 5, 6, 7], 3, async (n) => {
      started.push(n);
      running += 1;
      peak = Math.max(peak, running);
      await tick();
      running -= 1;
    });
    expect(count).toBe(7);
    expect(started).toEqual([1, 2, 3, 4, 5, 6, 7]);
    expect(peak).toBe(3);
  });

  it("starts nothing new once asked to stop, but lets running items finish", async () => {
    const finished: number[] = [];
    let stop = false;
    const count = await runLimited(
      [1, 2, 3, 4, 5, 6],
      2,
      async (n) => {
        // The stop arrives while 1 and 2 are both on their way.
        if (n === 2) stop = true;
        await tick();
        finished.push(n);
      },
      () => stop,
    );
    expect(count).toBe(2);
    expect(finished.sort()).toEqual([1, 2]);
  });

  it("copes with odd limits and empty lists", async () => {
    const seen: number[] = [];
    expect(await runLimited([], 4, async () => undefined)).toBe(0);
    expect(await runLimited([1, 2], 0, async (n) => void seen.push(n))).toBe(2);
    expect(await runLimited([3], 10, async (n) => void seen.push(n))).toBe(1);
    expect(seen).toEqual([1, 2, 3]);
  });
});
