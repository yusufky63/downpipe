// Italian. Each screen's texts live in their own file, mirroring en/.
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
import { feeds, grabber, hosts } from "./web";

export const it: Catalog = { common, states, chrome, list, app, add, confirm, details, dialogs, settings, media, grabber, feeds, hosts };
