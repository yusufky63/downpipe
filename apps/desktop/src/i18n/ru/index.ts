// ru translation. Fills in the English structure in en/; see catalogs.test.ts.
import type { Catalog } from "../core";
import { add } from "./add";
import { builtin } from "./builtin";
import { chrome } from "./chrome";
import { common, states } from "./common";
import { confirm } from "./confirm";
import { details } from "./details";
import { dialogs } from "./dialogs";
import { errors } from "./errors";
import { app, list } from "./list";
import { media } from "./media";
import { settings } from "./settings";

export const ru: Catalog = { common, states, chrome, list, app, add, details, dialogs, settings, media, confirm, errors, builtin };
