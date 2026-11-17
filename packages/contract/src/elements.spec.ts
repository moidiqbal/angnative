import { describe, it, expect } from 'vitest';
import {
  ELEMENT_TAGS,
  ELEMENT_CONTRACTS,
  getContract,
  isKnownElement,
  validateElementStyles,
} from './elements';

describe('element contracts (§8 / D9 na- prefix)', () => {
  it('exposes the canonical na-* tags', () => {
    expect(ELEMENT_TAGS.view).toBe('na-view');
    expect(ELEMENT_TAGS.text).toBe('na-text');
    expect(ELEMENT_TAGS.image).toBe('na-image');
    expect(ELEMENT_TAGS.scrollView).toBe('na-scroll-view');
    expect(ELEMENT_TAGS.list).toBe('na-list');
    expect(ELEMENT_TAGS.textInput).toBe('na-text-input');
    expect(ELEMENT_TAGS.pressable).toBe('na-pressable');
    expect(ELEMENT_TAGS.switch).toBe('na-switch');
    expect(ELEMENT_TAGS.activityIndicator).toBe('na-activity-indicator');
    expect(ELEMENT_TAGS.slot).toBe('na-slot');
  });

  it('has 10 element contracts', () => {
    expect(ELEMENT_CONTRACTS.size).toBe(10);
  });

  it('recognizes known elements', () => {
    expect(isKnownElement('na-view')).toBe(true);
    expect(isKnownElement('na-text')).toBe(true);
    expect(isKnownElement('native-view')).toBe(false); // legacy, not canonical
    expect(isKnownElement('div')).toBe(false);
  });

  it('returns contracts with defaults', () => {
    const view = getContract('na-view')!;
    expect(view.tag).toBe('na-view');
    expect(view.nodeType).toBe('View');
    expect(view.defaultStyles.flexDirection).toBe('column');
    expect(view.hostsText).toBe(false);

    const text = getContract('na-text')!;
    expect(text.nodeType).toBe('Text');
    expect(text.hostsText).toBe(true);

    const list = getContract('na-list')!;
    expect(list.recyclesView).toBe(true);
    expect(list.capabilities).toContain('recycle');
  });

  it('validates element styles', () => {
    const out = validateElementStyles('na-view', { width: '20', padding: '8' });
    expect(out.width).toBe('20px');
    expect(out.padding).toBe('8px');
  });

  it('rejects styles that do not apply to the element', () => {
    expect(() => validateElementStyles('na-image', { color: 'red' })).toThrow(
      /does not apply/,
    );
  });

  it('throws on unknown elements', () => {
    expect(getContract('na-not-an-element')).toBeUndefined();
    expect(() => validateElementStyles('na-ghost', {})).toThrow(/Unknown/);
  });
});
