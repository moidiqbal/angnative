/**
 * style.ts — AngNative CSS subset style table (spec §5.6).
 *
 * This is the exact set of style properties AngNative supports. It is
 * shared by the TypeScript contract layer and the Rust layout host
 * (taffy): the host lays out only these properties, and the platform
 * shells paint only these properties.
 *
 * Anything outside this table is rejected: unknown style keys throw
 * (spec §5.3 — a silent style drop is a silent device failure).
 */

export type StyleCategory = 'layout' | 'text' | 'border' | 'background' | 'effect';

export interface StyleProperty {
  /** CamelCase style key, e.g. `flexDirection`. */
  readonly name: string;
  readonly category: StyleCategory;
  /**
   * Element kinds this property applies to. `'all'` means every element
   * kind accepts it; otherwise only the listed kinds.
   */
  readonly appliesTo: 'all' | readonly string[];
  /** Whether the value inherits from the parent element (text properties). */
  readonly inherits: boolean;
  /** Initial / default value. */
  readonly initial: string;
  /** Parse and canonicalize a raw value. Throws on invalid input. */
  readonly parse: (value: string) => string;
}

/** Canonicalized style values: `Record<property, parsed value>`. */
export type StyleMap = Readonly<Record<string, string>>;

/* ---------- primitive parsers ---------- */

const DIMENSION = /^(\d+(?:\.\d+)?)px$/;
const PERCENT = /^(\d+(?:\.\d+)?)%$/;
const BARE_NUMBER = /^-?\d+(?:\.\d+)?$/;

export function parseDimension(value: string): string {
  const v = value.trim();
  if (v === '' || v === 'auto') return v;
  if (DIMENSION.test(v) || PERCENT.test(v)) return v;
  if (BARE_NUMBER.test(v)) return `${Number(v)}px`;
  throw new Error(`Invalid dimension: ${JSON.stringify(value)}`);
}

const HEX_COLOR = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/;
const NAMED_COLORS = new Set([
  'transparent', 'currentcolor', 'black', 'white', 'red', 'green', 'blue',
  'gray', 'grey', 'yellow', 'orange', 'purple', 'pink', 'brown',
]);
const RGB_COLOR = /^rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*(?:,\s*([0-9](?:\.[0-9]+)?|1|0?\.[0-9]+)?)\s*\)$/;

function clampByte(n: number, what: string): number {
  if (n < 0 || n > 255) throw new Error(`Invalid color: ${what}`);
  return n;
}

export function parseColor(value: string): string {
  const v = value.trim().toLowerCase();
  if (NAMED_COLORS.has(v) || HEX_COLOR.test(v)) return v;
  const m = RGB_COLOR.exec(v);
  if (!m) throw new Error(`Invalid color: ${JSON.stringify(value)}`);
  const r = clampByte(Number(m[1]), v);
  const g = clampByte(Number(m[2]), v);
  const b = clampByte(Number(m[3]), v);
  const a = m[4] === undefined ? 1 : Number(m[4]);
  if (a < 0 || a > 1) throw new Error(`Invalid color alpha: ${v}`);
  return a === 1 ? `rgb(${r},${g},${b})` : `rgba(${r},${g},${b},${a})`;
}

export function parseEnum(allowed: readonly string[]) {
  const lower = allowed.map((a) => a.toLowerCase());
  return (value: string) => {
    const v = value.trim().toLowerCase();
    if (!lower.includes(v)) {
      throw new Error(`Invalid value ${JSON.stringify(value)}, expected one of ${allowed.join(', ')}`);
    }
    return v;
  };
}

export function parseNumber(value: string): string {
  const v = value.trim();
  const n = Number(v);
  if (!BARE_NUMBER.test(v) || !Number.isFinite(n)) {
    throw new Error(`Invalid number: ${JSON.stringify(value)}`);
  }
  return String(n);
}

export function parsePercentage01(value: string): string {
  const n = Number(value.trim());
  if (!BARE_NUMBER.test(value.trim()) || n < 0 || n > 1) {
    throw new Error(`Invalid opacity (0..1): ${JSON.stringify(value)}`);
  }
  return String(n);
}

/* ---------- style table ---------- */

const LAYOUT_ENUM = {
  flexDirection: ['row', 'row-reverse', 'column', 'column-reverse'],
  flexWrap: ['nowrap', 'wrap', 'wrap-reverse'],
  justifyContent: ['flex-start', 'flex-end', 'center', 'space-between', 'space-around', 'space-evenly'],
  alignItems: ['stretch', 'flex-start', 'flex-end', 'center', 'baseline'],
  alignContent: ['stretch', 'flex-start', 'flex-end', 'center', 'space-between', 'space-around'],
  position: ['static', 'relative', 'absolute'],
  visibility: ['visible', 'hidden'],
} as const;

const TEXT_ENUM = {
  fontWeight: ['normal', 'bold', '100', '200', '300', '400', '500', '600', '700', '800', '900'],
  fontStyle: ['normal', 'italic', 'oblique'],
  textAlign: ['start', 'center', 'end', 'left', 'right', 'justify'],
  textDecoration: ['none', 'underline', 'line-through'],
  borderStyle: ['none', 'solid', 'dashed', 'dotted'],
} as const;

const TEXT_ELEMENTS = ['na-text', 'na-text-input'];
const INPUT_ELEMENTS = ['na-text-input'];

export const PROPERTY_TABLE: ReadonlyMap<string, StyleProperty> = new Map([
  // --- layout ---
  ['width', { name: 'width', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseDimension }],
  ['height', { name: 'height', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseDimension }],
  ['minWidth', { name: 'minWidth', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseDimension }],
  ['minHeight', { name: 'minHeight', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseDimension }],
  ['maxWidth', { name: 'maxWidth', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseDimension }],
  ['maxHeight', { name: 'maxHeight', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseDimension }],
  ['flex', { name: 'flex', category: 'layout', appliesTo: 'all', inherits: false, initial: '0', parse: parseNumber }],
  ['flexGrow', { name: 'flexGrow', category: 'layout', appliesTo: 'all', inherits: false, initial: '0', parse: parseNumber }],
  ['flexShrink', { name: 'flexShrink', category: 'layout', appliesTo: 'all', inherits: false, initial: '1', parse: parseNumber }],
  ['flexBasis', { name: 'flexBasis', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseDimension }],
  [
    'flexDirection',
    {
      name: 'flexDirection',
      category: 'layout',
      appliesTo: 'all',
      inherits: false,
      initial: 'row',
      parse: parseEnum(LAYOUT_ENUM.flexDirection),
    },
  ],
  [
    'flexWrap',
    {
      name: 'flexWrap',
      category: 'layout',
      appliesTo: 'all',
      inherits: false,
      initial: 'nowrap',
      parse: parseEnum(LAYOUT_ENUM.flexWrap),
    },
  ],
  [
    'justifyContent',
    {
      name: 'justifyContent',
      category: 'layout',
      appliesTo: 'all',
      inherits: false,
      initial: 'flex-start',
      parse: parseEnum(LAYOUT_ENUM.justifyContent),
    },
  ],
  [
    'alignItems',
    {
      name: 'alignItems',
      category: 'layout',
      appliesTo: 'all',
      inherits: false,
      initial: 'stretch',
      parse: parseEnum(LAYOUT_ENUM.alignItems),
    },
  ],
  [
    'alignSelf',
    {
      name: 'alignSelf',
      category: 'layout',
      appliesTo: 'all',
      inherits: false,
      initial: 'auto',
      parse: parseEnum([...LAYOUT_ENUM.alignItems, 'auto']),
    },
  ],
  [
    'alignContent',
    {
      name: 'alignContent',
      category: 'layout',
      appliesTo: 'all',
      inherits: false,
      initial: 'stretch',
      parse: parseEnum(LAYOUT_ENUM.alignContent),
    },
  ],
  ['gap', { name: 'gap', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['rowGap', { name: 'rowGap', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['columnGap', { name: 'columnGap', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['margin', { name: 'margin', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['marginTop', { name: 'marginTop', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['marginRight', { name: 'marginRight', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['marginBottom', { name: 'marginBottom', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['marginLeft', { name: 'marginLeft', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['marginX', { name: 'marginX', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['marginY', { name: 'marginY', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['padding', { name: 'padding', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['paddingTop', { name: 'paddingTop', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['paddingRight', { name: 'paddingRight', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['paddingBottom', { name: 'paddingBottom', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['paddingLeft', { name: 'paddingLeft', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['paddingX', { name: 'paddingX', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['paddingY', { name: 'paddingY', category: 'layout', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  [
    'position',
    {
      name: 'position',
      category: 'layout',
      appliesTo: 'all',
      inherits: false,
      initial: 'static',
      parse: parseEnum(LAYOUT_ENUM.position),
    },
  ],
  ['top', { name: 'top', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseDimension }],
  ['right', { name: 'right', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseDimension }],
  ['bottom', { name: 'bottom', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseDimension }],
  ['left', { name: 'left', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseDimension }],
  ['aspectRatio', { name: 'aspectRatio', category: 'layout', appliesTo: 'all', inherits: false, initial: 'auto', parse: parseNumber }],

  // --- text ---
  [
    'color',
    { name: 'color', category: 'text', appliesTo: TEXT_ELEMENTS, inherits: true, initial: 'black', parse: parseColor },
  ],
  [
    'fontSize',
    { name: 'fontSize', category: 'text', appliesTo: TEXT_ELEMENTS, inherits: true, initial: '16px', parse: parseDimension },
  ],
  [
    'fontWeight',
    {
      name: 'fontWeight',
      category: 'text',
      appliesTo: TEXT_ELEMENTS,
      inherits: true,
      initial: 'normal',
      parse: parseEnum(TEXT_ENUM.fontWeight),
    },
  ],
  [
    'fontStyle',
    {
      name: 'fontStyle',
      category: 'text',
      appliesTo: TEXT_ELEMENTS,
      inherits: true,
      initial: 'normal',
      parse: parseEnum(TEXT_ENUM.fontStyle),
    },
  ],
  [
    'fontFamily',
    { name: 'fontFamily', category: 'text', appliesTo: TEXT_ELEMENTS, inherits: true, initial: 'system-ui', parse: (v) => v.trim() },
  ],
  [
    'textAlign',
    {
      name: 'textAlign',
      category: 'text',
      appliesTo: TEXT_ELEMENTS,
      inherits: true,
      initial: 'start',
      parse: parseEnum(TEXT_ENUM.textAlign),
    },
  ],
  [
    'textDecoration',
    {
      name: 'textDecoration',
      category: 'text',
      appliesTo: TEXT_ELEMENTS,
      inherits: true,
      initial: 'none',
      parse: parseEnum(TEXT_ENUM.textDecoration),
    },
  ],
  [
    'lineHeight',
    {
      name: 'lineHeight',
      category: 'text',
      appliesTo: TEXT_ELEMENTS,
      inherits: true,
      initial: 'normal',
      parse: (v) => (v.trim() === 'normal' ? 'normal' : parseDimension(v)),
    },
  ],
  [
    'letterSpacing',
    {
      name: 'letterSpacing',
      category: 'text',
      appliesTo: TEXT_ELEMENTS,
      inherits: true,
      initial: 'normal',
      parse: (v) => (v.trim() === 'normal' ? 'normal' : parseDimension(v)),
    },
  ],

  // --- border ---
  ['borderWidth', { name: 'borderWidth', category: 'border', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['borderWidthTop', { name: 'borderWidthTop', category: 'border', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['borderWidthRight', { name: 'borderWidthRight', category: 'border', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['borderWidthBottom', { name: 'borderWidthBottom', category: 'border', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['borderWidthLeft', { name: 'borderWidthLeft', category: 'border', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension }],
  ['borderColor', { name: 'borderColor', category: 'border', appliesTo: 'all', inherits: false, initial: 'black', parse: parseColor }],
  ['borderColorTop', { name: 'borderColorTop', category: 'border', appliesTo: 'all', inherits: false, initial: 'black', parse: parseColor }],
  ['borderColorRight', { name: 'borderColorRight', category: 'border', appliesTo: 'all', inherits: false, initial: 'black', parse: parseColor }],
  ['borderColorBottom', { name: 'borderColorBottom', category: 'border', appliesTo: 'all', inherits: false, initial: 'black', parse: parseColor }],
  ['borderColorLeft', { name: 'borderColorLeft', category: 'border', appliesTo: 'all', inherits: false, initial: 'black', parse: parseColor }],
  [
    'borderRadius',
    { name: 'borderRadius', category: 'border', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension },
  ],
  [
    'borderRadiusTopLeft',
    { name: 'borderRadiusTopLeft', category: 'border', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension },
  ],
  [
    'borderRadiusTopRight',
    { name: 'borderRadiusTopRight', category: 'border', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension },
  ],
  [
    'borderRadiusBottomRight',
    { name: 'borderRadiusBottomRight', category: 'border', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension },
  ],
  [
    'borderRadiusBottomLeft',
    { name: 'borderRadiusBottomLeft', category: 'border', appliesTo: 'all', inherits: false, initial: '0px', parse: parseDimension },
  ],
  [
    'borderStyle',
    {
      name: 'borderStyle',
      category: 'border',
      appliesTo: 'all',
      inherits: false,
      initial: 'none',
      parse: parseEnum(TEXT_ENUM.borderStyle),
    },
  ],

  // --- background ---
  [
    'backgroundColor',
    { name: 'backgroundColor', category: 'background', appliesTo: 'all', inherits: false, initial: 'transparent', parse: parseColor },
  ],

  // --- effect ---
  [
    'opacity',
    { name: 'opacity', category: 'effect', appliesTo: 'all', inherits: false, initial: '1', parse: parsePercentage01 },
  ],
  [
    'visibility',
    {
      name: 'visibility',
      category: 'effect',
      appliesTo: 'all',
      inherits: true,
      initial: 'visible',
      parse: parseEnum(LAYOUT_ENUM.visibility),
    },
  ],
]);

export function getStyleProperty(name: string): StyleProperty | undefined {
  return PROPERTY_TABLE.get(name);
}

export function isKnownStyle(name: string): boolean {
  return PROPERTY_TABLE.has(name);
}

export function parseStyleValue(name: string, value: string): string {
  const prop = getStyleProperty(name);
  if (!prop) throw new Error(`Unknown style property: ${name}`);
  return prop.parse(value);
}

/**
 * Validate and canonicalize a style map for a given element kind.
 *
 * Throws on unknown keys (spec §5.3) and on values that don't parse.
 * Returns a new map with canonical values; initial values are not filled in.
 */
export function validateStyleMap(
  styles: Record<string, string>,
  elementKind: string,
): StyleMap {
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(styles)) {
    const prop = getStyleProperty(key);
    if (!prop) {
      throw new Error(`Unknown style property "${key}" for element "${elementKind}"`);
    }
    if (prop.appliesTo !== 'all' && !prop.appliesTo.includes(elementKind)) {
      throw new Error(
        `Style property "${key}" does not apply to element "${elementKind}"`,
      );
    }
    out[key] = prop.parse(value);
  }
  return out as StyleMap;
}
