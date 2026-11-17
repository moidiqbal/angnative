/**
 * NaScrollViewComponent — native scroll view (`<na-scroll-view>`).
 *
 * Maps to `na-scroll-view` / Rust `NodeType::ScrollView`. A scrollable
 * container that accepts any children.
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
  selector: 'na-scroll-view',
  standalone: true,
  template: `<ng-content></ng-content>`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NaScrollViewComponent implements OnInit {
  private readonly renderer = inject(AngNativeRendererFactory2).createRenderer(
    null,
    null,
  );
  private readonly element = inject(ElementRef).nativeElement as NativeElement;

  @Input() width?: string;
  @Input() height?: string;

  ngOnInit(): void {
    if (this.width) this.renderer.setStyle(this.element, 'width', this.width);
    if (this.height) this.renderer.setStyle(this.element, 'height', this.height);
  }
}




