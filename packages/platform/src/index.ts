/**
 * @angnative/platform — DOM-less Angular platform for native rendering.
 *
 * Provides `provideNativeAngular()` and `bootstrapNativeAngularApp()` so an
 * Angular 22+ standalone app runs on AngNative's custom renderer instead of
 * the browser DOM — the same app code targets Android, iOS, Desktop, and Web.
 */

export {
  bootstrapNativeAngularApp,
  type NativeAngularBootstrapOptions,
} from './bootstrap';

export {
  provideNativeAngular,
  provideNativeRendererFactory,
  type NativeAngularConfig,
  type FlushHandler,
  AngNativeRendererFactory2,
  AngNativeRenderer,
} from './native-angular';

export { provideNativeAnimations } from './native-animations';
