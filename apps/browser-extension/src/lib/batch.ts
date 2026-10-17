/**
 * Runs `task` over `items` with at most `limit` running at once, in order
 * of the list. Each hand-over starts a native host process, so a page with
 * hundreds of links must not start them all together. `shouldStop` is
 * checked before each new item: items already running finish, the rest
 * are never started. Resolves with how many items were started.
 */
export async function runLimited<T>(
  items: readonly T[],
  limit: number,
  task: (item: T, index: number) => Promise<void>,
  shouldStop: () => boolean = () => false,
): Promise<number> {
  let next = 0;
  const lane = async (): Promise<void> => {
    while (next < items.length && !shouldStop()) {
      const index = next;
      next += 1;
      await task(items[index] as T, index);
    }
  };
  const lanes = Math.max(1, Math.min(Math.floor(limit) || 1, items.length));
  await Promise.all(Array.from({ length: lanes }, lane));
  return next;
}
