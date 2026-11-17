/**
 * NaTextInputComponent — native text input (`<na-text-input>`).
 *
 * Maps to `na-text-input` / Rust `NodeType::TextInput`. Two-way text binding
 * via `ngModel`; emits (input) on native text change.
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
  selector: 'na-text-input',
  standalone: true,
  template: ``,
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '(input)': 'onInput($event)',
  },
})
export class NaTextInputComponent implements OnInit {
  private readonly renderer = inject(AngNativeRendererFactory2).createRenderer(
    null,
    null,
  );
  private readonly element = inject(ElementRef).nativeElement as NativeElement;

  @Input() value = '';
  @Input() placeholder?: string;
  @Input() disabled = false;
  @Output() readonly input = new EventEmitter<string>();

  ngOnInit(): void {
    this.renderer.setValue(this.element, this.value);
    if (this.placeholder) {
      this.renderer.setProperty(this.element, 'placeholder', this.placeholder);
    }
    this.renderer.setProperty(this.element, 'disabled', this.disabled);

    const unlisten = this.renderer.listen(this.element, 'text-input', (event: any) => {
      const next = event?.text ?? event?.value ?? '';
      this.value = next;
      this.input.emit(next);
    });

    (this.element as any).__textInputUnlisten = unlisten;
  }

  onInput(_event: any): void {
    // Native text-input events arrive via the listen callback above.
  }

  /** Programmatic value update (ngModel writes here). */
  writeValue(value: string): void {
    this.value = value ?? '';
    this.renderer.setValue(this.element, this.value);
  }
}




