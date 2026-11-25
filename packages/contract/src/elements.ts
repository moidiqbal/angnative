/**
 * elements.ts — AngNative element contracts.
 *
 * Canonical element set (spec §8, `na-` prefix per identity D-9 / spec D9).
 * Every element has a contract: tag name, Rust NodeType, default styles,
 * allowed children, and the capabilities it declares.
 *
 * Contracts are consumed by:
 *   - @angnative/core renderer (validate + create nodes)
 *   - @angnative/common (built-in primitive implementations)
 *   - the shared conformance suite (semantics per element)
 *   - Rust host-view (ViewFactory trait mapping)
 */

import type { StyleProperty, StyleMap } from './style';
import { validateStyleMap } from './style';

/** Canonical element tags — `na-` prefix (identity D-9). */
export const ELEMENT_TAGS = {
  view: 'na-view',
  text: 'na-text',
  image: 'na-image',
  scrollView: 'na-scroll-view',
  list: 'na-list',
  textInput: 'na-text-input',
  pressable: 'na-pressable',
  switch: 'na-switch',
  activityIndicator: 'na-activity-indicator',
  candlestick: 'na-candlestick',
  slot: 'na-slot',
} as const;

export type ElementTag = (typeof ELEMENT_TAGS)[keyof typeof ELEMENT_TAGS];

/** Element kinds mirroring Rust `NodeType` (crates/angnative-core/src/node.rs). */
export type ElementKind = 'view' | 'text' | 'button' | 'image' | 'scroll-view' | 'text-input' | 'custom';

export interface ElementContract {
  readonly tag: ElementTag;
  readonly kind: ElementKind;
  readonly nodeType: string; // Rust NodeType name as string
  /** Default styles applied when the element is created (canonical values). */
  readonly defaultStyles: StyleMap;
  /** Elements allowed as direct children. 'any' = no restriction. */
  readonly allowedChildren: 'any' | readonly ElementTag[];
  /** Capabilities this element declares (data only in M1). */
  readonly capabilities: readonly string[];
  /** Whether this element recycles a native view slot (e.g. na-list row). */
  readonly recyclesView: boolean;
  /** Whether the element hosts text content directly (na-text, na-text-input). */
  readonly hostsText: boolean;
}

/* ---------- shared style presets ---------- */

const VIEW_DEFAULTS: StyleMap = {
  width: 'auto',
  height: 'auto',
  flexDirection: 'column',
};

const TEXT_DEFAULTS: StyleMap = {
  width: 'auto',
  height: 'auto',
  fontSize: '16px',
  color: 'black',
};

const BUTTON_DEFAULTS: StyleMap = {
  ...VIEW_DEFAULTS,
  flexDirection: 'row',
  alignItems: 'center',
  justifyContent: 'center',
  borderRadius: '6px',
  borderWidth: '1px',
  borderColor: '#cccccc',
};

const SCROLL_DEFAULTS: StyleMap = {
  width: 'auto',
  height: 'auto',
};

const LIST_DEFAULTS: StyleMap = {
  width: 'auto',
  height: 'auto',
};

const INPUT_DEFAULTS: StyleMap = {
  width: 'auto',
  height: 'auto',
  fontSize: '16px',
  color: 'black',
  borderWidth: '1px',
  borderColor: '#cccccc',
  borderRadius: '4px',
  padding: '8px',
};

const PRESSABLE_DEFAULTS: StyleMap = {
  ...VIEW_DEFAULTS,
  flexDirection: 'row',
  alignItems: 'center',
};

const SWITCH_DEFAULTS: StyleMap = {
  width: '44px',
  height: '24px',
};

const ACTIVITY_DEFAULTS: StyleMap = {
  width: '36px',
  height: '36px',
};

const CANDLESTICK_DEFAULTS: StyleMap = {
  width: '320px',
  height: '220px',
};

const SLOT_DEFAULTS: StyleMap = {
  width: 'auto',
  height: 'auto',
};

/* ---------- contracts ---------- */

export const ELEMENT_CONTRACTS: ReadonlyMap<ElementTag, ElementContract> = new Map([
  [
    ELEMENT_TAGS.view,
    {
      tag: ELEMENT_TAGS.view,
      kind: 'view',
      nodeType: 'View',
      defaultStyles: VIEW_DEFAULTS,
      allowedChildren: 'any',
      capabilities: [],
      recyclesView: false,
      hostsText: false,
    },
  ],
  [
    ELEMENT_TAGS.text,
    {
      tag: ELEMENT_TAGS.text,
      kind: 'text',
      nodeType: 'Text',
      defaultStyles: TEXT_DEFAULTS,
      allowedChildren: 'any',
      capabilities: [],
      recyclesView: false,
      hostsText: true,
    },
  ],
  [
    ELEMENT_TAGS.image,
    {
      tag: ELEMENT_TAGS.image,
      kind: 'image',
      nodeType: 'Image',
      defaultStyles: {
        width: 'auto',
        height: 'auto',
      },
      allowedChildren: [],
      capabilities: [],
      recyclesView: false,
      hostsText: false,
    },
  ],
  [
    ELEMENT_TAGS.scrollView,
    {
      tag: ELEMENT_TAGS.scrollView,
      kind: 'scroll-view',
      nodeType: 'ScrollView',
      defaultStyles: SCROLL_DEFAULTS,
      allowedChildren: 'any',
      capabilities: ['scroll'],
      recyclesView: false,
      hostsText: false,
    },
  ],
  [
    ELEMENT_TAGS.list,
    {
      tag: ELEMENT_TAGS.list,
      kind: 'view',
      nodeType: 'Custom:na-list', // tagged custom: native shells materialize a list view
      defaultStyles: LIST_DEFAULTS,
      allowedChildren: ['na-view', 'na-slot'],
      capabilities: ['recycle'],
      recyclesView: true,
      hostsText: false,
    },
  ],
  [
    ELEMENT_TAGS.textInput,
    {
      tag: ELEMENT_TAGS.textInput,
      kind: 'text-input',
      nodeType: 'TextInput',
      defaultStyles: INPUT_DEFAULTS,
      allowedChildren: [],
      capabilities: ['text-input', 'focus'],
      recyclesView: false,
      hostsText: true,
    },
  ],
  [
    ELEMENT_TAGS.pressable,
    {
      tag: ELEMENT_TAGS.pressable,
      kind: 'button',
      nodeType: 'Button',
      defaultStyles: PRESSABLE_DEFAULTS,
      allowedChildren: 'any',
      capabilities: ['press'],
      recyclesView: false,
      hostsText: false,
    },
  ],
  [
    ELEMENT_TAGS.switch,
    {
      tag: ELEMENT_TAGS.switch,
      kind: 'button',
      nodeType: 'Custom:na-switch',
      defaultStyles: SWITCH_DEFAULTS,
      allowedChildren: [],
      capabilities: ['toggle'],
      recyclesView: false,
      hostsText: false,
    },
  ],
  [
    ELEMENT_TAGS.activityIndicator,
    {
      tag: ELEMENT_TAGS.activityIndicator,
      kind: 'custom',
      nodeType: 'Custom:na-activity-indicator',
      defaultStyles: ACTIVITY_DEFAULTS,
      allowedChildren: [],
      capabilities: [],
      recyclesView: false,
      hostsText: false,
    },
  ],
  [
    ELEMENT_TAGS.candlestick,
    {
      tag: ELEMENT_TAGS.candlestick,
      kind: 'custom',
      nodeType: 'Custom:na-candlestick', // tagged custom: native shells draw an OHLC canvas
      defaultStyles: CANDLESTICK_DEFAULTS,
      allowedChildren: [],
      capabilities: ['chart'],
      recyclesView: false,
      hostsText: false,
    },
  ],
  [
    ELEMENT_TAGS.slot,
    {
      tag: ELEMENT_TAGS.slot,
      kind: 'view',
      nodeType: 'View',
      defaultStyles: SLOT_DEFAULTS,
      allowedChildren: 'any',
      capabilities: ['slot'],
      recyclesView: false,
      hostsText: false,
    },
  ],
]);

export function getContract(tag: string): ElementContract | undefined {
  return ELEMENT_CONTRACTS.get(tag as ElementTag);
}

export function isKnownElement(tag: string): boolean {
  return ELEMENT_CONTRACTS.has(tag as ElementTag);
}

/**
 * Validate a style map against an element contract's applicable properties.
 * Uses the shared style table; throws on unknown or inapplicable keys (§5.3).
 */
export function validateElementStyles(
  tag: string,
  styles: Record<string, string>,
): StyleMap {
  const contract = getContract(tag);
  if (!contract) throw new Error(`Unknown element: ${tag}`);
  return validateStyleMap(styles, tag);
}
