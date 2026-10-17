// The libraries the client ships, as vite-plugin-credits.ts writes them to `credits.json` at build
// time: the Rust crates the app links and the JS packages that end up in its bundle.
export interface CreditPackage {
  name: string
  version: string
  // An SPDX expression as the package declares it, e.g. "MIT OR Apache-2.0".
  license?: string
  description?: string
  authors: string[]
  repository?: string
  homepage?: string
  // Indexes into Credits.licenseTexts: the package's LICENSE/COPYING/NOTICE files, often several
  // for a dual-licensed crate. Identical texts are stored once.
  licenseTexts: number[]
}

export interface Credits {
  rust: CreditPackage[]
  js: CreditPackage[]
  licenseTexts: string[]
  // A dev server lists only the client's direct JS dependencies: there's no bundle to read yet.
  partialJs?: boolean
}
