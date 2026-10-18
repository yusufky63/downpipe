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
import { playlist } from "./playlist";
import { remote } from "./remote";
import { settings } from "./settings";
import { sites } from "./sites";
import { feeds, grabber } from "./web";

export const ru: Catalog = { common, states, chrome, list, app, add, details, dialogs, settings, sites, media, confirm, errors, builtin, remote, grabber, feeds, playlist };
