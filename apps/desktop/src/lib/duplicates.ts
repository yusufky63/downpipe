// The question for a link that is already in the list, and the answer
// "Don't ask again" keeps in the settings.
import type { DuplicateAnswer, Settings } from "./api";

/** IDM's three answers to "this address is already in the list". */
export type DuplicateChoice = Exclude<DuplicateAnswer, "ask">;

export const DUPLICATE_CHOICES: readonly DuplicateChoice[] = ["copy", "overwrite", "existing"];

/** The answer to give without asking, or null to ask. */
export function rememberedAnswer(settings: Pick<Settings, "duplicate_answer">): DuplicateChoice | null {
  const answer = settings.duplicate_answer;
  return (DUPLICATE_CHOICES as readonly string[]).includes(answer) ? (answer as DuplicateChoice) : null;
}
