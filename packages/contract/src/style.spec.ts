import { describe, it, expect } from 'vitest';
import {
  PROPERTY_TABLE,
  isKnownStyle,
  parseStyleValue,
  validateStyleMap,
  parseDimension,
  parseColor,
  parseNumber,
  parsePercentage01,
} from './style';

describe('style table (§5.6)', () => {
  it('knows layout properties', () => {
    expect(isKnownStyle('flexDirection')).toBe(true);
    expect(isKnownStyle('width')).toBe(true);
    expect(isKnownStyle('gap')).toBe(true);
    expect(isKnownStyle('borderRadius')).toBe(true);
    expect(isKnownStyle('color')).toBe(true);
    expect(isKnownStyle('opacity')).toBe(true);
  });

  it('rejects unknown properties', () => {
    expect(isKnownStyle('border-box')).toBe(false);
    expect(isKnownStyle('display')).toBe(false);
    expect(isKnownStyle('flex-direction')).toBe(false); // camelCase only
    expect(() => parseStyleValue('notAStyle', '1px')).toThrow(/Unknown style/);
  });

  it('parses dimensions', () => {
    expect(parseDimension('10px')).toBe('10px');
    expect(parseDimension('50%')).toBe('50%');
    expect(parseDimension('7')).toBe('7px');
    expect(parseDimension('auto')).toBe('auto');
    expect(() => parseDimension('10em')).toThrow();
    expect(() => parseDimension('abc')).toThrow();
  });

  it('parses colors', () => {
    expect(parseColor('#fff')).toBe('#fff');
    expect(parseColor('#ff0000ff')).toBe('#ff0000ff');
    expect(parseColor('transparent')).toBe('transparent');
    expect(parseColor('rgb(255, 0, 0)')).toBe('rgb(255, 0, 0)');
    expect(parseColor('rgba(255, 0, 0, 0.5)')).toBe('rgba(255, 0, 0, 0.5)');
    expect(() => parseColor('#ggg')).toThrow();
    expect(() => parseColor('notacolor')).toThrow();
  });

  it('parses numbers and opacity', () => {
    expect(parseNumber('3')).toBe('3');
    expect(parseNumber('1.5')).toBe('1.5');
    expect(() => parseNumber('px')).toThrow();
    expect(parsePercentage01('0.5')).toBe('0.5');
    expect(parsePercentage01('1')).toBe('1');
    expect(() => parsePercentage01('1.5')).toThrow();
    expect(() => parsePercentage01('-0.1')).toThrow();
  });

  it('validates and canonicalizes a style map', () => {
    const out = validateStyleMap(
      { width: '10', height: '50%', color: '#f00', flexDirection: 'row' },
      'na-view',
    );
    expect(out.width).toBe('10px');
    expect(out.height).toBe('50%');
    expect(out.color).toBe('#f00');
    expect(out.flexDirection).toBe('row');
  });

  it('rejects unknown keys loudly (§5.3)', () => {
    expect(() =>
      validateStyleMap({ bogus: '1px' } as any, 'na-view'),
    ).toThrow(/Unknown style/);
  });

  it('rejects text-only styles on non-text elements', () => {
    expect(() => validateStyleMap({ fontSize: '14px' }, 'na-view')).toThrow(
      /does not apply/,
    );
  });

  it('applies enum parse with case-insensitivity', () => {
    expect(parseStyleValue('flexDirection', 'ROW')).toBe('row');
    expect(parseStyleValue('justifyContent', 'CENTER')).toBe('center');
    expect(() => parseStyleValue('position', 'sticky')).toThrow();
  });
});
