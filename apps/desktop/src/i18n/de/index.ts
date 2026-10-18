// German. Each screen's texts live in their own file, mirroring en/; see catalogs.test.ts.
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
import { post } from "./post";
import { remote } from "./remote";
import { settings } from "./settings";
import { sites } from "./sites";
import { torrent } from "./torrent";
import { feeds, grabber } from "./web";

export const de: Catalog = { common, states, chrome, list, app, add, confirm, details, dialogs, settings, sites, media, errors, builtin, remote, grabber, feeds, post, playlist, torrent };
