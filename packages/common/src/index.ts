import { NaViewComponent } from './na-view.component';
import { NaTextComponent } from './na-text.component';
import { NaPressableComponent } from './na-pressable.component';
import { NaButtonComponent } from './na-button.component';
import { NaTextInputComponent } from './na-text-input.component';
import { NaImageComponent } from './na-image.component';
import { NaScrollViewComponent } from './na-scroll-view.component';

export {
  NaViewComponent,
  NaTextComponent,
  NaPressableComponent,
  NaButtonComponent,
  NaTextInputComponent,
  NaImageComponent,
  NaScrollViewComponent,
};

export const NATIVE_COMPONENTS = [
  NaViewComponent,
  NaTextComponent,
  NaPressableComponent,
  NaButtonComponent,
  NaTextInputComponent,
  NaImageComponent,
  NaScrollViewComponent,
] as const;
