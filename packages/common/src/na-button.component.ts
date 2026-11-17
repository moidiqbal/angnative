/**
 * NaButtonComponent — native button (`<na-button>`).
 *
 * Maps to `na-button` / Rust `NodeType::Button`. A pressable element with
 * button styling defaults and an (click) output.
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
  selector: 'na-button',
  standalone: true,
  template: `<ng-content></ng-content>`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NaButtonComponent implements OnInit {
  private readonly renderer = inject(AngNativeRendererFactory2).createRenderer(
    null,
    null,
  );
  private readonly element = inject(ElementRef).nativeElement as NativeElement;

  @Input() disabled = false;
  @Input() text?: string;
  @Output() readonly click = new EventEmitter<void>();

  ngOnInit(): void {
    if (this.text !== undefined && this.text !== null) {
      this.renderer.setValue(this.element, String(this.text));
    }

    this.renderer.setProperty(this.element, 'disabled', this.disabled);

    const unlisten = this.renderer.listen(this.element, 'press', (_event) => {
      if (this.disabled) return;
      this.click.emit();
    });

    (this.element as any).__pressUnlisten = unlisten;
  }
}




