// French. Each screen's texts live in their own file, mirroring en/; see catalogs.test.ts.
import type { Catalog } from "../core";
import { common, results } from "./common";
import { errors } from "./errors";
import { links } from "./links";
import { media } from "./media";
import { menus } from "./menus";
import { notifications } from "./notifications";
import { options } from "./options";
import { overlay } from "./overlay";
import { popup } from "./popup";
import { status } from "./status";

export const fr: Catalog = { common, results, errors, status, popup, media, options, links, notifications, menus, overlay };
