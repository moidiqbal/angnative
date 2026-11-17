/**
 * NaPressableComponent — touchable native element (`<na-pressable>`).
 *
 * Maps to `na-pressable` / Rust `NodeType::Button`. Emits (press) on tap/click.
 */

import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  inject,
  Input,
  Output,
  type OnInit,
  EventEmitter,
} from '@angular/core';
import {
  AngNativeRendererFactory2,
  NativeElement,
} from '@angnative/core';

@Component({
  selector: 'na-pressable',
  standalone: true,
  template: `<ng-content></ng-content>`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NaPressableComponent implements OnInit {
  private readonly renderer = inject(AngNativeRendererFactory2).createRenderer(
    null,
    null,
  );
  private readonly element = inject(ElementRef).nativeElement as NativeElement;

  @Input() disabled = false;
  @Output() readonly press = new EventEmitter<void>();

  ngOnInit(): void {
    this.renderer.setProperty(this.element, 'disabled', this.disabled);

    // Listen for native press events from the platform shell / Rust host.
    const unlisten = this.renderer.listen(this.element, 'press', (_event) => {
      if (this.disabled) return;
      this.press.emit();
    });

    // Store the unlisten function so the component can clean up.
    (this.element as any).__pressUnlisten = unlisten;
  }

  private setStyle(key: string, value: string | undefined): void {
    if (value === undefined || value === null) return;
    this.renderer.setStyle(this.element, key, value);
  }
}




