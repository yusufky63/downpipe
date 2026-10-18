// Every language's texts by code. English is complete by definition; the
// others are checked against it in catalogs.test.ts.
import type { Catalog, Locale } from "./core";
import { de } from "./de";
import { en } from "./en";
import { es } from "./es";
import { fr } from "./fr";
import { it } from "./it";
import { ja } from "./ja";
import { ptBR } from "./pt-BR";
import { ru } from "./ru";
import { tr } from "./tr";
import { zhCN } from "./zh-CN";

export const CATALOGS: Record<Locale, Catalog> = { en, tr, es, de, fr, it, "pt-BR": ptBR, ru, "zh-CN": zhCN, ja };
