/**
 * Rustdoc JSON → Astro Markdown generator.
 *
 * Runs `cargo +nightly rustdoc` for each configured crate, then transforms
 * the JSON output into markdown pages for the Astro content collection at
 * `docs/src/content/docs/api-reference/`.
 */

import { execSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { readFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";

// ─── Configuration ───────────────────────────────────────────────────────────

const DOCS_DIR = resolve(import.meta.dirname!, "..");
const PROJECT_ROOT = resolve(DOCS_DIR, "..");
const OUTPUT_DIR = join(DOCS_DIR, "src/content/docs/api-reference");
const NAV_OUTPUT = join(DOCS_DIR, "src/data/api-navigation.json");
const TARGET_DOC = join(PROJECT_ROOT, "target/doc");

interface CrateConfig {
  /** Cargo package name (used in `cargo rustdoc -p`) */
  package: string;
  /** Filename of the generated JSON (underscores, no extension) */
  jsonName: string;
  /** URL slug used in page paths */
  slug: string;
  /** Extra cargo flags */
  cargoFlags?: string;
}

const CRATES: CrateConfig[] = [
  { package: "ion-skill", jsonName: "ion_skill", slug: "ion-skill", cargoFlags: "--all-features" },
  { package: "ionem", jsonName: "ionem", slug: "ionem", cargoFlags: "--all-features" },
  { package: "scenario", jsonName: "scenario", slug: "scenario" },
];

/** Trait impls worth showing. Everything else (Send, Sync, Any, …) is noise. */
const NOTABLE_TRAITS = new Set([
  "Display", "Debug", "Clone", "Default", "Error",
  "From", "Into", "FromStr",
  "Iterator", "IntoIterator",
  "Deref", "DerefMut", "AsRef", "AsMut",
  "Serialize", "Deserialize",
  "PartialEq", "Eq", "PartialOrd", "Ord", "Hash",
]);

// ─── Rustdoc JSON Types (format v57) ─────────────────────────────────────────

interface RustdocJson {
  root: number;
  crate_version: string | null;
  includes_private: boolean;
  index: Record<string, Item>;
  paths: Record<string, PathEntry>;
  external_crates: Record<string, ExternalCrate>;
  format_version: number;
}

interface PathEntry {
  crate_id: number;
  path: string[];
  kind: string;
}

interface ExternalCrate {
  name: string;
  html_root_url: string | null;
}

interface Item {
  id: number;
  crate_id: number;
  name: string | null;
  span: { filename: string; begin: [number, number]; end: [number, number] } | null;
  visibility: string;
  docs: string | null;
  links: Record<string, number>;
  attrs: unknown[];
  deprecation: unknown | null;
  inner: Record<string, unknown>;
}

interface ModuleInner {
  is_crate: boolean;
  items: number[];
  is_stripped: boolean;
}

interface StructInner {
  kind: { plain?: { fields: number[]; has_stripped_fields: boolean }; unit?: unknown; tuple?: number[] };
  generics: Generics;
  impls: number[];
}

interface EnumInner {
  generics: Generics;
  has_stripped_variants: boolean;
  variants: number[];
  impls: number[];
}

interface FunctionInner {
  sig: FunctionSig;
  generics: Generics;
  header: FunctionHeader;
  has_body: boolean;
}

interface FunctionSig {
  inputs: [string, RustType][];
  output: RustType | null;
  is_c_variadic: boolean;
}

interface FunctionHeader {
  is_const: boolean;
  is_unsafe: boolean;
  is_async: boolean;
  abi: string;
}

interface Generics {
  params: GenericParam[];
  where_predicates: WherePredicate[];
}

interface GenericParam {
  name: string;
  kind: Record<string, unknown>;
}

interface WherePredicate {
  bound_predicate?: {
    type: RustType;
    bounds: TraitBoundWrapper[];
  };
  [key: string]: unknown;
}

interface TraitBoundWrapper {
  trait_bound?: TraitBound;
  [key: string]: unknown;
}

interface TraitBound {
  trait: ResolvedPath;
  generic_params: GenericParam[];
  modifier: string;
}

interface ResolvedPath {
  path: string;
  id: number;
  args: GenericArgs | null;
}

interface GenericArgs {
  angle_bracketed?: {
    args: GenericArg[];
    constraints: Constraint[];
  };
  parenthesized?: {
    inputs: RustType[];
    output: RustType | null;
  };
}

interface GenericArg {
  type?: RustType;
  lifetime?: string;
  const?: { type: RustType; value: string };
}

interface Constraint {
  name: string;
  args: GenericArgs | null;
  binding: { equality?: { type: RustType }; [key: string]: unknown } | null;
}

interface ImplInner {
  is_unsafe: boolean;
  generics: Generics;
  provided_trait_methods: string[];
  trait: ResolvedPath | null;
  for: RustType;
  items: number[];
  is_negative: boolean;
  is_synthetic: boolean;
  blanket_impl: unknown | null;
}

interface UseInner {
  source: string;
  name: string;
  id: number | null;
  is_glob: boolean;
}

interface VariantInner {
  kind: string | { struct?: { fields: number[]; has_stripped_fields: boolean }; tuple?: number[] };
  discriminant: { value: string; expr: string } | null;
}

// A RustType is a single-key object
type RustType = Record<string, unknown>;

// ─── Type Renderer ───────────────────────────────────────────────────────────

function renderType(t: RustType | null | undefined, depth = 0): string {
  if (!t || depth > 20) return "…";

  if ("primitive" in t) return t.primitive as string;
  if ("generic" in t) return t.generic as string;

  if ("resolved_path" in t) {
    const rp = t.resolved_path as ResolvedPath;
    const shortPath = shortenPath(rp.path);
    const args = renderGenericArgs(rp.args, depth);
    return `${shortPath}${args}`;
  }

  if ("borrowed_ref" in t) {
    const br = t.borrowed_ref as { lifetime: string | null; is_mutable: boolean; type: RustType };
    const lt = br.lifetime ? `${br.lifetime} ` : "";
    const mut = br.is_mutable ? "mut " : "";
    return `&${lt}${mut}${renderType(br.type, depth + 1)}`;
  }

  if ("slice" in t) return `[${renderType(t.slice as RustType, depth + 1)}]`;

  if ("array" in t) {
    const arr = t.array as { type: RustType; len: string };
    return `[${renderType(arr.type, depth + 1)}; ${arr.len}]`;
  }

  if ("tuple" in t) {
    const items = t.tuple as RustType[];
    if (items.length === 0) return "()";
    return `(${items.map((i) => renderType(i, depth + 1)).join(", ")})`;
  }

  if ("raw_pointer" in t) {
    const rp = t.raw_pointer as { is_mutable: boolean; type: RustType };
    return `*${rp.is_mutable ? "mut" : "const"} ${renderType(rp.type, depth + 1)}`;
  }

  if ("impl_trait" in t) {
    const bounds = t.impl_trait as TraitBoundWrapper[];
    return `impl ${renderBounds(bounds, depth)}`;
  }

  if ("dyn_trait" in t) {
    const dt = t.dyn_trait as { traits: TraitBoundWrapper[]; lifetime: string | null };
    const traits = dt.traits || (t.dyn_trait as TraitBoundWrapper[]);
    if (Array.isArray(traits)) {
      return `dyn ${renderBounds(traits, depth)}`;
    }
    return "dyn …";
  }

  if ("qualified_path" in t) {
    const qp = t.qualified_path as {
      name: string;
      args: GenericArgs | null;
      self_type: RustType;
      trait: ResolvedPath;
    };
    const selfType = renderType(qp.self_type, depth + 1);
    const traitPath = shortenPath(qp.trait.path);
    return `<${selfType} as ${traitPath}>::${qp.name}`;
  }

  if ("function_pointer" in t) {
    const fp = t.function_pointer as { sig: FunctionSig; header: FunctionHeader; generic_params: unknown[] };
    const inputs = fp.sig.inputs.map(([, ty]) => renderType(ty, depth + 1)).join(", ");
    const output = fp.sig.output ? ` -> ${renderType(fp.sig.output, depth + 1)}` : "";
    return `fn(${inputs})${output}`;
  }

  if ("infer" in t) return "_";
  if ("pat" in t) {
    const pat = t.pat as { name: string; type: RustType };
    return renderType(pat.type, depth + 1);
  }

  return "…";
}

function renderGenericArgs(args: GenericArgs | null | undefined, depth: number): string {
  if (!args) return "";

  if (args.angle_bracketed) {
    const ab = args.angle_bracketed;
    const parts: string[] = [];
    for (const arg of ab.args) {
      if (arg.type) parts.push(renderType(arg.type, depth + 1));
      else if (arg.lifetime) parts.push(arg.lifetime);
      else if (arg.const) parts.push(arg.const.value || "…");
    }
    for (const c of ab.constraints) {
      const binding = c.binding?.equality
        ? ` = ${renderType(c.binding.equality.type, depth + 1)}`
        : "";
      parts.push(`${c.name}${binding}`);
    }
    if (parts.length === 0) return "";
    return `<${parts.join(", ")}>`;
  }

  if (args.parenthesized) {
    const p = args.parenthesized;
    const inputs = p.inputs.map((i) => renderType(i, depth + 1)).join(", ");
    const output = p.output ? ` -> ${renderType(p.output, depth + 1)}` : "";
    return `(${inputs})${output}`;
  }

  return "";
}

function renderBounds(bounds: TraitBoundWrapper[], depth: number): string {
  return bounds
    .filter((b) => b.trait_bound)
    .map((b) => {
      const tb = b.trait_bound!;
      const path = shortenPath(tb.trait.path);
      const args = renderGenericArgs(tb.trait.args, depth);
      return `${path}${args}`;
    })
    .join(" + ");
}

function shortenPath(path: string): string {
  // Strip serde internals
  if (path.startsWith("_serde::")) {
    const last = path.split("::").pop()!;
    // Map known serde internal names
    if (last === "__private228") return "serde";
    return last;
  }
  // Use last segment for std paths and well-known types
  const segments = path.split("::");
  if (segments.length === 1) return path;

  const last = segments[segments.length - 1];
  // Keep short for std types
  if (segments[0] === "std" || segments[0] === "core" || segments[0] === "alloc") {
    // For io::Error, path::Path etc keep the module prefix
    if (["Error", "Result"].includes(last) && segments.length >= 3) {
      return `${segments[segments.length - 2]}::${last}`;
    }
    return last;
  }
  return last;
}

// ─── Signature Renderer ──────────────────────────────────────────────────────

function renderFunctionSig(name: string, fn_: FunctionInner, isMethod: boolean): string {
  const header = fn_.header;
  let prefix = "pub ";
  if (header.is_const) prefix += "const ";
  if (header.is_async) prefix += "async ";
  if (header.is_unsafe) prefix += "unsafe ";

  const params = fn_.sig.inputs.map(([paramName, paramType]) => {
    // Handle self parameters
    if (paramName === "self") {
      if ("generic" in paramType && paramType.generic === "Self") return "self";
      if ("borrowed_ref" in paramType) {
        const br = paramType.borrowed_ref as { is_mutable: boolean; type: RustType };
        if ("generic" in br.type && br.type.generic === "Self") {
          return br.is_mutable ? "&mut self" : "&self";
        }
      }
    }
    return `${paramName}: ${renderType(paramType)}`;
  });

  // Generics (skip synthetic impl Trait params)
  const genericParams = fn_.generics.params
    .filter((p) => !("type" in (p.kind as Record<string, unknown>) && ((p.kind as Record<string, unknown>).type as Record<string, unknown>)?.is_synthetic))
    .map((p) => p.name);
  const generics = genericParams.length > 0 ? `<${genericParams.join(", ")}>` : "";

  // Where clauses
  const whereClauses = renderWhereClauses(fn_.generics);

  const output = fn_.sig.output ? ` -> ${renderType(fn_.sig.output)}` : "";
  return `${prefix}fn ${name}${generics}(${params.join(", ")})${output}${whereClauses}`;
}

function renderWhereClauses(generics: Generics): string {
  const clauses: string[] = [];
  for (const pred of generics.where_predicates) {
    if (pred.bound_predicate) {
      const bp = pred.bound_predicate;
      const ty = renderType(bp.type);
      const bounds = renderBounds(bp.bounds, 0);
      if (bounds) clauses.push(`${ty}: ${bounds}`);
    }
  }
  if (clauses.length === 0) return "";
  return `\nwhere\n    ${clauses.join(",\n    ")}`;
}

// ─── Page Generators ─────────────────────────────────────────────────────────

interface PageOutput {
  slug: string;
  content: string;
}

interface NavItem {
  title: string;
  slug: string;
}

function getItemKind(item: Item): string {
  return Object.keys(item.inner)[0];
}

function getInner<T>(item: Item, kind: string): T {
  return item.inner[kind] as T;
}

/** Collect inherent methods and notable trait impls for a type's impl list. */
function collectImpls(implIds: number[], index: Record<string, Item>) {
  const methods: Item[] = [];
  const traitImpls: { traitName: string; traitPath: string }[] = [];

  for (const implId of implIds) {
    const implItem = index[String(implId)];
    if (!implItem || getItemKind(implItem) !== "impl") continue;
    const impl_ = getInner<ImplInner>(implItem, "impl");

    // Skip synthetic (auto traits) and blanket impls
    if (impl_.is_synthetic || impl_.blanket_impl) continue;

    if (!impl_.trait) {
      // Inherent impl — collect methods
      for (const methodId of impl_.items) {
        const method = index[String(methodId)];
        if (method && method.visibility === "public" && getItemKind(method) === "function") {
          methods.push(method);
        }
      }
    } else {
      // Trait impl — check if notable
      const traitName = shortenPath(impl_.trait.path);
      if (NOTABLE_TRAITS.has(traitName)) {
        const args = renderGenericArgs(impl_.trait.args, 0);
        traitImpls.push({ traitName, traitPath: `${traitName}${args}` });
      }
    }
  }

  return { methods, traitImpls };
}

function renderStructPage(item: Item, index: Record<string, Item>): string {
  const struct_ = getInner<StructInner>(item, "struct");
  let md = `## ${item.name}\n\n`;

  if (item.docs) md += `${item.docs}\n\n`;

  // Fields
  if (struct_.kind.plain) {
    const fields = struct_.kind.plain.fields
      .map((fid) => index[String(fid)])
      .filter((f): f is Item => !!f && f.visibility === "public");

    if (fields.length > 0) {
      md += `### Fields\n\n`;
      md += `| Name | Type | Description |\n|------|------|-------------|\n`;
      for (const field of fields) {
        const fieldType = renderType(field.inner.struct_field as RustType);
        const desc = field.docs ? field.docs.split("\n")[0] : "" ;
        md += `| \`${field.name}\` | \`${fieldType}\` | ${desc} |\n`;
      }
      md += "\n";
    }

    if (struct_.kind.plain.has_stripped_fields) {
      md += `*…and private fields*\n\n`;
    }
  } else if (struct_.kind.unit) {
    md += `*Unit struct*\n\n`;
  } else if (struct_.kind.tuple) {
    const tupleFields = (struct_.kind.tuple as number[])
      .map((fid) => index[String(fid)])
      .filter(Boolean)
      .map((f) => renderType(f!.inner.struct_field as RustType));
    if (tupleFields.length > 0) {
      md += `\`\`\`rust\npub struct ${item.name}(${tupleFields.join(", ")})\n\`\`\`\n\n`;
    }
  }

  // Methods and trait impls
  const { methods, traitImpls } = collectImpls(struct_.impls, index);
  md += renderMethods(methods);
  md += renderTraitImpls(traitImpls);

  return md;
}

function renderEnumPage(item: Item, index: Record<string, Item>): string {
  const enum_ = getInner<EnumInner>(item, "enum");
  let md = `## ${item.name}\n\n`;

  if (item.docs) md += `${item.docs}\n\n`;

  // Variants
  const variants = enum_.variants
    .map((vid) => index[String(vid)])
    .filter((v): v is Item => !!v);

  if (variants.length > 0) {
    md += `### Variants\n\n`;
    for (const variant of variants) {
      const variantInner = getInner<VariantInner>(variant, "variant");
      let variantSig = variant.name!;

      if (typeof variantInner.kind === "object" && variantInner.kind) {
        if (variantInner.kind.struct) {
          const fields = variantInner.kind.struct.fields
            .map((fid: number) => index[String(fid)])
            .filter(Boolean);
          const fieldStrs = fields.map((f: Item) => `${f.name}: ${renderType(f.inner.struct_field as RustType)}`);
          variantSig += ` { ${fieldStrs.join(", ")} }`;
        } else if (variantInner.kind.tuple) {
          const fields = (variantInner.kind.tuple as number[])
            .map((fid: number) => index[String(fid)])
            .filter(Boolean)
            .map((f: Item) => renderType(f.inner.struct_field as RustType));
          variantSig += `(${fields.join(", ")})`;
        }
      }

      md += `- **\`${variantSig}\`**`;
      if (variant.docs) md += ` — ${variant.docs.split("\n")[0]}`;
      md += "\n";
    }
    md += "\n";
  }

  // Methods and trait impls
  const { methods, traitImpls } = collectImpls(enum_.impls, index);
  md += renderMethods(methods);
  md += renderTraitImpls(traitImpls);

  return md;
}

function renderFreeFunction(item: Item): string {
  const fn_ = getInner<FunctionInner>(item, "function");
  let md = `## ${item.name}\n\n`;
  md += `\`\`\`rust\n${renderFunctionSig(item.name!, fn_, false)}\n\`\`\`\n\n`;
  if (item.docs) md += `${item.docs}\n\n`;
  return md;
}

function renderMethods(methods: Item[]): string {
  if (methods.length === 0) return "";
  let md = `### Methods\n\n`;
  for (const method of methods) {
    const fn_ = getInner<FunctionInner>(method, "function");
    md += `#### \`${method.name}\`\n\n`;
    md += `\`\`\`rust\n${renderFunctionSig(method.name!, fn_, true)}\n\`\`\`\n\n`;
    if (method.docs) md += `${method.docs}\n\n`;
  }
  return md;
}

function renderTraitImpls(traitImpls: { traitName: string; traitPath: string }[]): string {
  if (traitImpls.length === 0) return "";
  let md = `### Trait Implementations\n\n`;
  for (const ti of traitImpls) {
    md += `- \`${ti.traitPath}\`\n`;
  }
  md += "\n";
  return md;
}

// ─── Module & Crate Generators ───────────────────────────────────────────────

function generateModulePage(
  module: Item,
  index: Record<string, Item>,
  crateSlug: string,
  modulePath: string,
): PageOutput {
  const mod_ = getInner<ModuleInner>(module, "module");
  const title = modulePath;
  const description = extractDescription(module.docs);

  let md = `---\ntitle: "${title}"\ndescription: "${escapeYaml(description)}"\norder: 999\n---\n\n`;

  if (module.docs) md += `${module.docs}\n\n`;

  // Collect items by kind
  const structs: Item[] = [];
  const enums: Item[] = [];
  const functions: Item[] = [];
  const reexports: Item[] = [];

  for (const itemId of mod_.items) {
    const item = index[String(itemId)];
    if (!item || item.visibility !== "public") continue;
    const kind = getItemKind(item);
    switch (kind) {
      case "struct": structs.push(item); break;
      case "enum": enums.push(item); break;
      case "function": functions.push(item); break;
      case "use": reexports.push(item); break;
    }
  }

  // Render each section
  if (structs.length > 0) {
    for (const s of structs) {
      md += renderStructPage(s, index);
      md += "---\n\n";
    }
  }

  if (enums.length > 0) {
    for (const e of enums) {
      md += renderEnumPage(e, index);
      md += "---\n\n";
    }
  }

  if (functions.length > 0) {
    for (const f of functions) {
      md += renderFreeFunction(f);
      md += "---\n\n";
    }
  }

  // Trim trailing ---
  md = md.replace(/---\n\n$/, "");

  const slug = `api-reference/${crateSlug}/${moduleNameToSlug(module.name!)}`;
  return { slug, content: md };
}

function generateCrateIndex(
  root: Item,
  index: Record<string, Item>,
  crateSlug: string,
  crateVersion: string | null,
): { indexPage: PageOutput; modulePages: PageOutput[]; navItems: NavItem[] } {
  const mod_ = getInner<ModuleInner>(root, "module");
  const description = extractDescription(root.docs);

  let md = `---\ntitle: "${crateSlug}"\ndescription: "${escapeYaml(description)}"\norder: 100\n---\n\n`;

  if (crateVersion) md += `*Version ${crateVersion}*\n\n`;
  if (root.docs) md += `${root.docs}\n\n`;

  // Collect child modules and re-exports
  const modules: Item[] = [];
  const reexports: Item[] = [];

  for (const itemId of mod_.items) {
    const item = index[String(itemId)];
    if (!item || item.visibility !== "public") continue;
    const kind = getItemKind(item);
    if (kind === "module") modules.push(item);
    if (kind === "use") reexports.push(item);
  }

  // Module table
  if (modules.length > 0) {
    md += `## Modules\n\n`;
    md += `| Module | Description |\n|--------|-------------|\n`;
    for (const m of modules) {
      const desc = extractDescription(m.docs);
      const slug = moduleNameToSlug(m.name!);
      md += `| [${m.name}](/docs/api-reference/${crateSlug}/${slug}) | ${desc} |\n`;
    }
    md += "\n";
  }

  // Re-exports
  if (reexports.length > 0) {
    md += `## Re-exports\n\n`;
    for (const re of reexports) {
      const use_ = getInner<UseInner>(re, "use");
      md += `- \`pub use ${use_.source}\` as **${use_.name}**\n`;
    }
    md += "\n";
  }

  // Generate module pages
  const modulePages: PageOutput[] = [];
  const navItems: NavItem[] = [
    { title: "Overview", slug: `api-reference/${crateSlug}` },
  ];

  for (const m of modules) {
    const modulePath = `${crateSlug}::${m.name}`;
    const page = generateModulePage(m, index, crateSlug, modulePath);
    modulePages.push(page);
    navItems.push({ title: m.name!, slug: page.slug });

    // Handle nested submodules
    const subMod = getInner<ModuleInner>(m, "module");
    for (const subId of subMod.items) {
      const subItem = index[String(subId)];
      if (!subItem || subItem.visibility !== "public" || getItemKind(subItem) !== "module") continue;
      const subPath = `${crateSlug}::${m.name}::${subItem.name}`;
      const subPage = generateModulePage(subItem, index, crateSlug, subPath);
      // Adjust slug for nesting
      subPage.slug = `api-reference/${crateSlug}/${moduleNameToSlug(m.name!)}/${moduleNameToSlug(subItem.name!)}`;
      modulePages.push(subPage);
      navItems.push({ title: `${m.name}::${subItem.name}`, slug: subPage.slug });
    }
  }

  const indexPage: PageOutput = {
    slug: `api-reference/${crateSlug}`,
    content: md,
  };

  return { indexPage, modulePages, navItems };
}

// ─── Utilities ───────────────────────────────────────────────────────────────

function escapeYaml(s: string): string {
  return s.replace(/"/g, '\\"').replace(/\n/g, " ");
}

function moduleNameToSlug(name: string): string {
  return name.replace(/_/g, "-");
}

/** Extract first non-heading, non-empty line from docs for use as YAML description. */
function extractDescription(docs: string | null): string {
  if (!docs) return "";
  for (const line of docs.split("\n")) {
    const trimmed = line.trim();
    if (trimmed && !trimmed.startsWith("#") && !trimmed.startsWith("```")) {
      return trimmed;
    }
  }
  return "";
}

function ensureDir(dir: string) {
  mkdirSync(dir, { recursive: true });
}

function writePage(page: PageOutput) {
  const filePath = join(OUTPUT_DIR, `${page.slug.replace("api-reference/", "")}.md`);
  ensureDir(dirname(filePath));
  writeFileSync(filePath, page.content, "utf-8");
  console.log(`  wrote ${filePath.replace(DOCS_DIR + "/", "")}`);
}

// ─── Rustdoc Runner ──────────────────────────────────────────────────────────

function runRustdoc() {
  // Check nightly is available
  try {
    execSync("rustup run nightly rustc --version", { stdio: "pipe", cwd: PROJECT_ROOT });
  } catch {
    console.error("Error: Rust nightly toolchain not found.");
    console.error("Install it with: rustup toolchain install nightly");
    process.exit(1);
  }

  for (const crate of CRATES) {
    const jsonPath = join(TARGET_DOC, `${crate.jsonName}.json`);
    console.log(`Generating rustdoc JSON for ${crate.package}...`);
    const flags = crate.cargoFlags || "";
    try {
      execSync(
        `cargo +nightly rustdoc -p ${crate.package} ${flags} -- -Z unstable-options --output-format json`,
        { stdio: "inherit", cwd: PROJECT_ROOT },
      );
    } catch {
      console.error(`Error: Failed to generate rustdoc JSON for ${crate.package}`);
      process.exit(1);
    }

    if (!existsSync(jsonPath)) {
      console.error(`Error: Expected JSON output at ${jsonPath} but file not found`);
      process.exit(1);
    }
  }
}

// ─── Main ────────────────────────────────────────────────────────────────────

function main() {
  console.log("=== Rustdoc JSON → Astro Docs Generator ===\n");

  // Step 1: Generate rustdoc JSON
  runRustdoc();

  // Step 2: Clear output directory
  if (existsSync(OUTPUT_DIR)) {
    rmSync(OUTPUT_DIR, { recursive: true });
  }
  ensureDir(OUTPUT_DIR);

  // Step 3: Transform each crate
  const allNavSections: { title: string; items: NavItem[] }[] = [];

  for (const crate of CRATES) {
    const jsonPath = join(TARGET_DOC, `${crate.jsonName}.json`);
    console.log(`\nTransforming ${crate.package}...`);

    const data: RustdocJson = JSON.parse(readFileSync(jsonPath, "utf-8"));
    const root = data.index[String(data.root)];

    if (!root) {
      console.error(`Error: Could not find root item in ${jsonPath}`);
      process.exit(1);
    }

    const { indexPage, modulePages, navItems } = generateCrateIndex(
      root,
      data.index,
      crate.slug,
      data.crate_version,
    );

    writePage(indexPage);
    for (const page of modulePages) {
      writePage(page);
    }

    allNavSections.push({ title: `API: ${crate.slug}`, items: navItems });
  }

  // Step 4: Write navigation JSON
  writeFileSync(NAV_OUTPUT, JSON.stringify(allNavSections, null, 2), "utf-8");
  console.log(`\nWrote navigation: ${NAV_OUTPUT.replace(DOCS_DIR + "/", "")}`);

  console.log("\nDone!");
}

main();
