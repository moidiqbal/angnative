# Angular Runtime Requirements for Native Execution

## Executive Summary

This document identifies the exact Angular runtime requirements for executing Angular applications inside an embedded JavaScript engine without a browser DOM.

## Angular Version: 22.x (AngNative Target)

## Core Runtime Dependencies

### 1. Angular Platform Abstractions

#### Required Platform Providers

```typescript
// Minimal platform providers for AngNative
[
  // Change detection
  provideZonelessChangeDetection(),  // or provideZoneChangeDetection({ eventCoalescing: true })
  
  // Renderer
  { provide: RendererFactory2, useClass: AngNativeRendererFactory2 },
  { provide: Renderer2, useFactory: (f: RendererFactory2) => f.createRenderer(null, null), deps: [RendererFactory2] },
  
  // Platform services
  { provide: PlatformLocation, useClass: NativePlatformLocation },
  { provide: LocationStrategy, useClass: PathLocationStrategy },
  { provide: DOCUMENT, useValue: createDocumentShim() },
  
  // DI
  { provide: EnvironmentInjector, useFactory: ... },
  
  // Error handling
  { provide: ErrorHandler, useClass: NativeErrorHandler },
  
  // Testability (dev only)
  { provide: TestabilityRegistry, useClass: TestabilityRegistry },
]
```

#### Renderer2 Interface Requirements

Angular's `Renderer2` requires these methods:

```typescript
interface Renderer2 {
  // Node creation
  createElement(name: string, namespace?: string): any;
  createComment(value: string): any;
  createText(value: string): any;
  
  // Tree mutation
  appendChild(parent: any, newChild: any): void;
  insertBefore(parent: any, newChild: any, refChild: any): void;
  removeChild(parent: any, oldChild: any): void;
  selectRootElement(selectorOrNode: string | any, preserveContent?: boolean): any;
  parentNode(node: any): any;
  nextSibling(node: any): any;
  
  // Properties/Attributes
  setProperty(el: any, name: string, value: any): void;
  setAttribute(el: any, name: string, value: string): void;
  removeAttribute(el: any, name: string): void;
  addClass(el: any, name: string): void;
  removeClass(el: any, name: string): void;
  setStyle(el: any, style: string, value: any, flags?: RendererStyleFlags2): void;
  removeStyle(el: any, style: string, flags?: RendererStyleFlags2): void;
  
  // Values
  setValue(node: any, value: string): void;
  
  // Events
  listen(target: any, eventName: string, callback: (event: any) => boolean | void): () => void;
  
  // Data
  data: { [key: string]: any };
  
  // Destruction
  destroyNode?: (node: any) => void;
  destroy(): void;
}
```

#### RendererFactory2 Interface

```typescript
interface RendererFactory2 {
  createRenderer(hostElement: any, type: RendererType2 | null): Renderer2;
  begin(): void;
  end(): void;
  whenRenderingDone(): Promise<any>;
}
```

### 2. Dependency Injection

#### Required DI Infrastructure

```typescript
// Core DI tokens Angular expects
- Injector
- InjectorType
- InjectionToken
- InjectFlags
- InjectOptions
- Provider
- StaticProvider
- ClassProvider
- FactoryProvider
- ValueProvider
- ExistingProvider
- EnvironmentProviders
- makeEnvironmentProviders
- inject()
- InjectFlags
- INJECTOR_SCOPE
```

**AngNative Status**: ✅ Angular DI works without DOM - pure TypeScript

### 3. Change Detection

#### Zoneless Change Detection (Angular 18+)

Angular 18+ supports zoneless via:
```typescript
provideZonelessChangeDetection()
```

**Requirements**:
- `scheduler.scheduleMicroTask(fn)` - queues microtasks
- `scheduler.scheduleMacroTask(fn)` - queues macrotasks
- Event dispatch via `EventTarget` or equivalent

**AngNative Implementation**:
```typescript
// QuickJS microtask queue integration
class QuickJSScheduler {
  scheduleMicroTask(fn: () => void): void {
    // Queue in QuickJS Promise job queue
    runtime.eval(`queueMicrotask(() => { ${fn.toString()}() })`);
  }
}
```

#### Zoneless Requirements (from Angular Source)

Angular's zoneless implementation requires:
1. `scheduler.scheduleMicroTask` - for `queueMicrotask`
2. `scheduler.scheduleMacroTask` - for `setTimeout` (optional)
3. Event listener wrapping for automatic change detection
4. `ApplicationRef.tick()` for manual triggering

**AngNative Status**: Need to implement microtask scheduler for QuickJS

### 4. Angular Signals

Signals work without Zone.js:
```typescript
const count = signal(0);
const double = computed(() => count() * 2);
effect(() => console.log(count()));
```

**Requirements**:
- `signal()` - creates reactive value
- `computed()` - derived reactive value
- `effect()` - side effect on signal changes
- Automatic change detection scheduling

**AngNative Status**: ✅ Signals are pure TypeScript - work without DOM

### 5. Component Lifecycle

#### Required Lifecycle Hooks

```typescript
interface OnInit { ngOnInit(): void; }
interface OnDestroy { ngOnDestroy(): void; }
interface OnChanges { ngOnChanges(changes: SimpleChanges): void; }
interface DoCheck { ngDoCheck(): void; }
interface AfterContentInit { ngAfterContentInit(): void; }
interface AfterContentChecked { ngAfterContentChecked(): void; }
interface AfterViewInit { ngAfterViewInit(): void; }
interface AfterViewChecked { ngAfterViewChecked(): void; }
```

**AngNative Status**: ✅ Pure TypeScript - no DOM required

### 6. Template Rendering (Ivy)

#### AOT Compilation Output

Angular 22 AOT produces:
```typescript
// Component definition
static ɵcmp = ɵɵdefineComponent({
  type: MyComponent,
  selectors: [['my-component']],
  decls: 5,  // DOM nodes
  vars: 3,   // bindings
  template: function(rf, ctx) {
    if (rf & 1) {  // Creation phase
      ɵɵelementStart(0, 'div');
      ɵɵtext(1, 'Hello ');
      ɵɵelement(2, 'span');
      ɵɵelementEnd();
    }
    if (rf & 2) {  // Update phase
      ɵɵadvance(1);
      ɵɵtextInterpolate(ctx.name);
    }
  },
  dependencies: [],
  encapsulation: ViewEncapsulation.None
});
```

**Ivy Instructions Used**:
- `ɵɵdefineComponent` - Component definition
- `ɵɵelementStart/End` - Element creation
- `ɵɵelement` - Self-closing element
- `ɵɵtext` - Text node
- `ɵɵtextInterpolate` - Text interpolation
- `ɵɵproperty` - Property binding
- `ɵɵattribute` - Attribute binding
- `ɵɵstyleProp` - Style binding
- `ɵɵclassProp` - Class binding
- `ɵɵlistener` - Event listener
- `ɵɵadvance` - Move to next binding
- `ɵɵpropertyInterpolate` - Property interpolation

**Key Insight**: These are **function calls** that ultimately call `Renderer2` methods.

### 7. Event Handling

#### Angular Event Binding

Template: `<button (click)="onClick()">`
AOT Output:
```typescript
ɵɵlistener('click', function() { ctx.onClick(); });
```

**Renderer2.listen() Called**:
```typescript
renderer.listen(buttonElement, 'click', (event) => {
  // Angular wraps handler
  handler(event);
});
```

**AngNative Requirement**:
- `Renderer2.listen()` must register native event listener
- Native event → JavaScript → Angular handler
- Event object must have minimal properties: `target`, `type`, `preventDefault()`

### 8. Content Projection

#### `<ng-content>` Support

Angular uses:
- `ɵɵprojectionDef` - Projection definition
- `ɵɵprojection` - Render projected content

**AngNative Requirement**: Content projection nodes in shadow tree

### 9. Structural Directives

#### `*ngIf`, `*ngFor`, `*ngSwitch`

These manipulate the view tree:
- `*ngIf` - Creates/destroys embedded views
- `*ngFor` - Creates repeated embedded views with tracking
- `*ngSwitch` - Switches between embedded views

**AngNative Requirement**: View container operations in shadow tree

### 10. Dependency Injection in Components

#### Constructor Injection

```typescript
@Component({...})
export class MyComponent {
  constructor(
    private service: MyService,
    @Inject(TOKEN) private value: string,
    @Optional() private optional?: OptionalService
  ) {}
```

**AngNative Status**: ✅ Pure DI - works without DOM

### 11. Pipes

#### Pure/Impure Pipes

```typescript
@Pipe({ name: 'format', pure: true })
export class FormatPipe implements PipeTransform {
  transform(value: any, ...args: any[]): any { ... }
}
```

**AngNative Status**: ✅ Pure TypeScript

---

## Platform Shim Requirements

### Minimal Browser APIs Needed

Based on Angular 22 source analysis, these browser APIs are referenced:

#### 1. Global Objects
```typescript
// Required
globalThis
Promise
Map
Set
WeakMap
WeakSet
Symbol
Reflect
Proxy
Error
TypeError
ReferenceError

// For Zone.js (if used)
Zone
zone.js
```

#### 2. Timer APIs (for Zone.js or async)
```typescript
setTimeout
clearTimeout
setInterval
clearInterval
queueMicrotask
requestAnimationFrame
cancelAnimationFrame
```

#### 3. Event System
```typescript
Event
CustomEvent
MouseEvent
KeyboardEvent
EventTarget
addEventListener
removeEventListener
dispatchEvent
preventDefault
stopPropagation
```

#### 4. DOM-Like (Minimal)
```typescript
document
  createElement(tagName)
  createTextNode(text)
  createComment(text)
  querySelector(selector)
  querySelectorAll(selector)
  getElementById(id)
  body
  documentElement
  head

Element
  setAttribute(name, value)
  getAttribute(name)
  removeAttribute(name)
  hasAttribute(name)
  style (CSSStyleDeclaration)
  className
  id
  tagName
  addEventListener
  removeEventListener
  dispatchEvent
  parentNode
  appendChild
  removeChild
  insertBefore
  replaceChild
  firstChild
  lastChild
  nextSibling
  previousSibling
  ownerDocument

Text
  nodeValue
  textContent
  parentNode

Comment
  nodeValue
```

#### 5. CSSStyleDeclaration (Minimal)
```typescript
style.setProperty(name, value)
style.removeProperty(name)
style.getPropertyValue(name)
style[name] = value  // camelCase
```

#### 6. Console
```typescript
console.log
console.warn
console.error
console.debug
console.info
```

---

## QuickJS Shim Implementation Plan

### Phase 1: Core Globals
```javascript
// In QuickJS context
globalThis = globalThis || this;
globalThis.Promise = Promise;
globalThis.queueMicrotask = queueMicrotask;
globalThis.setTimeout = setTimeout;
globalThis.clearTimeout = clearTimeout;
globalThis.console = console;
```

### Phase 2: Event System
```javascript
class EventTarget {
  listeners = new Map();
  addEventListener(type, handler) { ... }
  removeEventListener(type, handler) { ... }
  dispatchEvent(event) { ... }
}

class Event {
  constructor(type, options) { ... }
  preventDefault() { ... }
  stopPropagation() { ... }
}

class CustomEvent extends Event { ... }
```

### Phase 3: Document Shim
```javascript
const document = {
  createElement(tag) { ... },
  createTextNode(text) { ... },
  createComment(text) { ... },
  querySelector(sel) { return null; },
  querySelectorAll(sel) { return []; },
  getElementById(id) { return null; },
  body: null,
  documentElement: null,
  head: null,
};
```

### Phase 4: Element Shim
```javascript
function createElementShim(tag) {
  return {
    tagName: tag.toUpperCase(),
    nodeType: 1,
    style: {},
    className: '',
    id: '',
    setAttribute(name, value) { ... },
    getAttribute(name) { return null; },
    removeAttribute(name) { ... },
    addEventListener: EventTarget.prototype.addEventListener,
    removeEventListener: EventTarget.prototype.removeEventListener,
    dispatchEvent: EventTarget.prototype.dispatchEvent,
    parentNode: null,
    appendChild(child) { ... },
    removeChild(child) { ... },
    insertBefore(newNode, refNode) { ... },
    firstChild: null,
    lastChild: null,
    nextSibling: null,
    previousSibling: null,
    ownerDocument: document,
  };
}
```

---

## Verification Checklist

| Requirement | Status | Notes |
|-------------|--------|-------|
| Renderer2 implementation | ✅ Done | `packages/core/src/renderer.ts` |
| RendererFactory2 | ✅ Done | `packages/core/src/renderer.ts` |
| Angular DI | ✅ Works | No DOM needed |
| Zoneless change detection | ⚠️ Pending | Needs microtask scheduler |
| Signals | ✅ Works | Pure TypeScript |
| Component lifecycle | ✅ Works | Pure TypeScript |
| Ivy instruction execution | ⚠️ Pending | Needs JS engine integration |
| Event handling | ⚠️ Pending | Needs native→JS bridge |
| Template binding | ⚠️ Pending | Needs Renderer2 → Host |
| Content projection | ❌ Not started | View container ops |
| Structural directives | ❌ Not started | View container ops |
| Dependency injection | ✅ Works | No DOM needed |
| Pipes | ✅ Works | Pure TypeScript |
| Platform providers | ⚠️ Partial | Need `bootstrapApplication` integration |

---

## Next Steps

1. **Implement QuickJS microtask scheduler** for zoneless change detection
2. **Complete `install_api()`** with all Renderer2 methods
3. **Wire `bootstrapApplication`** to QuickJS runtime
4. **Load AOT bundle** into QuickJS context
5. **Verify component execution** with real traces

---

## References

- Angular Source: https://github.com/angular/angular (v22.x)
- Angular Zoneless: https://angular.dev/guide/zoneless
- Angular Signals: https://angular.dev/guide/signals
- Ivy Instructions: https://github.com/angular/angular/blob/main/packages/core/src/render3/instructions.ts
- Renderer2: https://angular.io/api/core/Renderer2