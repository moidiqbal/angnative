/**
 * @angnative/public contract API.
 *
 * Exports the element contracts (§8), the CSS-subset style table (§5.6),
 * and the versioned TS↔Rust command protocol — the boundary between the
 * Angular TypeScript layer and the Rust host.
 */

export {
  ELEMENT_TAGS,
  type ElementTag,
  type ElementKind,
  type ElementContract,
  ELEMENT_CONTRACTS,
  getContract,
  isKnownElement,
  validateElementStyles,
} from './elements';

export {
  type StyleProperty,
  type StyleMap,
  type StyleCategory,
  PROPERTY_TABLE,
  getStyleProperty,
  isKnownStyle,
  parseStyleValue,
  validateStyleMap,
  parseDimension,
  parseColor,
  parseEnum,
  parseNumber,
  parsePercentage01,
} from './style';

export {
  type Command,
  type CommandBatch,
  type CommandCreateNode,
  type CommandRemoveNode,
  type CommandInsertChild,
  type CommandRemoveChild,
  type CommandSetProperty,
  type CommandSetStyle,
  type CommandSetTextContent,
  PROTOCOL_VERSION,
  COMMAND_KINDS,
  createBatch,
  encodeBatch,
  decodeBatch,
  roundTrip,
} from './protocol';
