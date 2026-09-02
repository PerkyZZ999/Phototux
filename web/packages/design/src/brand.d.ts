/** Hand-written types for `brand.js`. Kept beside it so the sites get real
 *  completion instead of `any`, without adding a build step to a package that
 *  is three plain files. */

export interface Brand {
  name: string;
  tagline: string;
  description: string;
  version: string;
  licence: string;
  site: string;
  docs: string;
  repo: string;
  issues: string;
  discussions: string;
  authors: { name: string; role: string }[];
}

export interface Requirement {
  icon: string;
  label: string;
  detail: string;
}

export declare const BRAND: Brand;
export declare const REQUIREMENTS: Requirement[];
export declare const BUILD_COMMAND: string;
