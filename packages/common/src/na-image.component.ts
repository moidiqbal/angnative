/**
 * NaImageComponent — native image (`<na-image>`).
 *
 * Maps to `na-image` / Rust `NodeType::Image`. Loads from `src` URI.
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
  selector: 'na-image',
  standalone: true,
  template: ``,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NaImageComponent implements OnInit {
  private readonly renderer = inject(AngNativeRendererFactory2).createRenderer(
    null,
    null,
  );
  private readonly element = inject(ElementRef).nativeElement as NativeElement;

  @Input() src?: string;
  @Input() width?: string;
  @Input() height?: string;

  ngOnInit(): void {
    if (this.src) {
      this.renderer.setProperty(this.element, 'src', this.src);
    }
    if (this.width) this.renderer.setStyle(this.element, 'width', this.width);
    if (this.height) this.renderer.setStyle(this.element, 'height', this.height);
  }
}




