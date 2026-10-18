// Turkish. Each screen's texts live in their own file, in the shape of en/.
import type { Catalog } from "../core";
import { add } from "./add";
import { chrome } from "./chrome";
import { common, states } from "./common";
import { confirm } from "./confirm";
import { details } from "./details";
import { dialogs } from "./dialogs";
import { app, list } from "./list";
import { media } from "./media";
import { settings } from "./settings";
import { sites } from "./sites";

export const tr: Catalog = { common, states, chrome, list, app, add, confirm, details, dialogs, settings, sites, media };
