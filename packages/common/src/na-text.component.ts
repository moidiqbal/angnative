/**
 * NaTextComponent — native text (`<na-text>`).
 *
 * Maps to `na-text` / Rust `NodeType::Text`. Renders text content via
 * `renderer.setValue`, with @Input() text style bindings.
 *
 * The `text` input is reactive: signal/template changes after init are
 * pushed to the native element through an input setter.
 */

import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  inject,
  Input,
  type OnInit,
} from '@angular/core';
import {
  AngNativeRendererFactory2,
  NativeElement,
} from '@angnative/core';

@Component({
  selector: 'na-text',
  standalone: true,
  template: `<ng-content></ng-content>`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NaTextComponent implements OnInit {
  private readonly renderer = inject(AngNativeRendererFactory2).createRenderer(
    null,
    null,
  );
  private readonly element = inject(ElementRef).nativeElement as NativeElement;

  private _text?: string;

  @Input() set text(value: string | undefined) {
    this._text = value;
    this.renderer.setValue(this.element, String(value ?? ''));
  }
  get text(): string | undefined {
    return this._text;
  }

  @Input() color?: string;
  @Input() fontSize?: string;
  @Input() fontWeight?: 'normal' | 'bold' | '100' | '200' | '300' | '400' | '500' | '600' | '700' | '800' | '900';
  @Input() fontStyle?: 'normal' | 'italic' | 'oblique';
  @Input() fontFamily?: string;
  @Input() textAlign?: 'start' | 'center' | 'end' | 'left' | 'right' | 'justify';
  @Input() textDecoration?: 'none' | 'underline' | 'line-through';
  @Input() lineHeight?: string | 'normal';
  @Input() letterSpacing?: string | 'normal';

  ngOnInit(): void {
    if (this._text !== undefined && this._text !== null) {
      this.renderer.setValue(this.element, String(this._text));
    }

    this.setStyle('color', this.color);
    this.setStyle('fontSize', this.fontSize);
    this.setStyle('fontWeight', this.fontWeight);
    this.setStyle('fontStyle', this.fontStyle);
    this.setStyle('fontFamily', this.fontFamily);
    this.setStyle('textAlign', this.textAlign);
    this.setStyle('textDecoration', this.textDecoration);
    this.setStyle('lineHeight', this.lineHeight);
    this.setStyle('letterSpacing', this.letterSpacing);
  }

  private setStyle(key: string, value: string | undefined): void {
    if (value === undefined || value === null) return;
    this.renderer.setStyle(this.element, key, value);
  }
}




