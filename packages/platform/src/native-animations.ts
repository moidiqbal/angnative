/**
 * native-animations.ts — Angular animation providers for the native renderer.
 *
 * `provideAnimations()` from @angular/platform-browser always wraps
 * `DomRendererFactory2`, so it cannot drive AngNative's renderer. These
 * providers wire the same Angular `AnimationEngine` and
 * `AnimationRendererFactory` around `AngNativeRendererFactory2` instead.
 */

import {
  AnimationDriver,
  ɵAnimationEngine as AnimationEngine,
  ɵAnimationRendererFactory as AnimationRendererFactory,
  ɵAnimationStyleNormalizer as AnimationStyleNormalizer,
  ɵBaseAnimationRenderer as BaseAnimationRenderer,
  ɵWebAnimationsDriver as WebAnimationsDriver,
  ɵWebAnimationsStyleNormalizer as WebAnimationsStyleNormalizer,
} from '@angular/animations/browser';
import { DOCUMENT } from '@angular/common';
import {
  NgZone,
  RendererFactory2,
  type Provider,
  type RendererType2,
} from '@angular/core';
import { AngNativeRendererFactory2 } from '@angnative/core';

/**
 * Angular validates style properties against `document.body.style`, which the
 * native renderer does not have. Delegate keyframe construction to Angular's
 * Web Animations driver, but answer platform queries from the native tree.
 */
class NativeAnimationDriver implements AnimationDriver {
  private readonly delegate = new WebAnimationsDriver();

  constructor(private readonly factory: AngNativeRendererFactory2) {}

  validateStyleProperty(): boolean {
    return true;
  }

  validateAnimatableStyleProperty(): boolean {
    return true;
  }

  containsElement(elm1: unknown, elm2: unknown): boolean {
    // Angular asks whether an animated element is attached to `document.body`.
    // Native elements live in AngNative's shadow tree, not the DOM shim, so
    // treat the shim body as their document root. Otherwise the engine marks
    // them orphaned and downgrades every transition to a no-op player.
    if (elm1 && typeof elm1 === 'object' && this.isNativeElement(elm2)) {
      const candidate = elm1 as { tagName?: unknown };
      const shimBody = (globalThis as { document?: { body?: unknown } }).document?.body;
      if (candidate.tagName === 'BODY' || elm1 === shimBody) return true;
    }
    let current: unknown = elm2;
    while (current) {
      if (current === elm1) return true;
      current = this.getParentElement(current);
    }
    return false;
  }

  private isNativeElement(value: unknown): boolean {
    return !!value
      && typeof value === 'object'
      && typeof (value as { id?: unknown }).id === 'number'
      && 'nativeNodeType' in value;
  }

  getParentElement(element: unknown): unknown {
    if (!element || typeof element !== 'object') return null;
    const candidate = element as { parentNode?: unknown; parentId?: number };
    if (candidate.parentNode) return candidate.parentNode;
    if (typeof candidate.parentId === 'number') {
      return this.factory.getRenderer().getRegistry().get(candidate.parentId) ?? null;
    }
    return null;
  }

  query(): unknown[] {
    return [];
  }

  computeStyle(element: unknown, prop: string, defaultValue?: string): string {
    if (element && typeof element === 'object') {
      const styles = (element as { styles?: Map<string, string> }).styles;
      const value = styles?.get(prop);
      if (value !== undefined) return value;
    }
    return defaultValue ?? '';
  }

  animate(
    element: any,
    keyframes: Array<Map<string, string | number>>,
    duration: number,
    delay: number,
    easing: string,
    previousPlayers?: any[],
  ): any {
    return this.delegate.animate(element, keyframes, duration, delay, easing, previousPlayers);
  }
}

/**
 * Angular bootstraps a root component with a `null` renderer host. Stock
 * `AnimationRendererFactory` treats that as "not animatable" and falls back to
 * `BaseAnimationRenderer`, which silently ignores `@trigger` bindings. Supply a
 * single flat host object for that case; the native delegate ignores the host
 * argument, while the animation engine gets the non-null key it needs.
 */
class NativeAnimationRendererFactory extends AnimationRendererFactory {
  private readonly rootHost = Object.freeze({
    __angnativeAnimationRoot: true,
    nodeType: 1,
  });

  override createRenderer(hostElement: any, type: RendererType2): BaseAnimationRenderer {
    return super.createRenderer(hostElement ?? this.rootHost, type);
  }
}

/**
 * Providers for Angular's real animation engine over the native renderer.
 * Place after `provideNativeAngular()` so the animation-aware
 * `RendererFactory2` wins while the base factory stays injectable as the
 * delegate.
 */
export function provideNativeAnimations(): Provider[] {
  return [
    {
      provide: AnimationDriver,
      useFactory: (factory: AngNativeRendererFactory2) => new NativeAnimationDriver(factory),
      deps: [AngNativeRendererFactory2],
    },
    { provide: AnimationStyleNormalizer, useClass: WebAnimationsStyleNormalizer },
    {
      provide: AnimationEngine,
      useFactory: (
        doc: unknown,
        driver: AnimationDriver,
        normalizer: AnimationStyleNormalizer,
      ) => new AnimationEngine(doc as never, driver, normalizer),
      deps: [DOCUMENT, AnimationDriver, AnimationStyleNormalizer],
    },
    {
      provide: RendererFactory2,
      useFactory: (
        delegate: AngNativeRendererFactory2,
        engine: AnimationEngine,
        zone: NgZone,
      ) => new NativeAnimationRendererFactory(delegate, engine, zone),
      deps: [AngNativeRendererFactory2, AnimationEngine, NgZone],
    },
  ];
}
