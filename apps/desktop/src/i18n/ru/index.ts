// ru translation. Fills in the English structure in en/; see catalogs.test.ts.
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

export const ru: Catalog = { common, states, chrome, list, app, add, details, dialogs, settings, sites, media, confirm };
