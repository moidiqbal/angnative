/**
 * NaViewComponent — native view container (`<na-view>`).
 *
 * Maps to `na-view` / Rust `NodeType::View`. A layout container that accepts
 * any children and forwards @Input() style bindings to the native renderer.
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
  selector: 'na-view',
  standalone: true,
  template: `<ng-content></ng-content>`,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NaViewComponent implements OnInit {
  private readonly renderer = inject(AngNativeRendererFactory2).createRenderer(
    null,
    null,
  );
  private readonly element = inject(ElementRef).nativeElement as NativeElement;

  @Input() width?: string;
  @Input() height?: string;
  @Input() minWidth?: string;
  @Input() minHeight?: string;
  @Input() maxWidth?: string;
  @Input() maxHeight?: string;
  @Input() flex?: number;
  @Input() flexGrow?: number;
  @Input() flexShrink?: number;
  @Input() flexBasis?: string;
  @Input() flexDirection?: 'row' | 'row-reverse' | 'column' | 'column-reverse';
  @Input() flexWrap?: 'nowrap' | 'wrap' | 'wrap-reverse';
  @Input() justifyContent?:
    | 'flex-start'
    | 'flex-end'
    | 'center'
    | 'space-between'
    | 'space-around'
    | 'space-evenly';
  @Input() alignItems?: 'stretch' | 'flex-start' | 'flex-end' | 'center' | 'baseline';
  @Input() alignSelf?: 'auto' | 'stretch' | 'flex-start' | 'flex-end' | 'center' | 'baseline';
  @Input() alignContent?: 'stretch' | 'flex-start' | 'flex-end' | 'center' | 'space-between' | 'space-around';
  @Input() gap?: string;
  @Input() margin?: string;
  @Input() padding?: string;
  @Input() position?: 'static' | 'relative' | 'absolute';
  @Input() top?: string;
  @Input() right?: string;
  @Input() bottom?: string;
  @Input() left?: string;
  @Input() opacity?: number;
  @Input() backgroundColor?: string;
  @Input() borderWidth?: string;
  @Input() borderColor?: string;
  @Input() borderRadius?: string;

  ngOnInit(): void {
    this.setStyle('width', this.width);
    this.setStyle('height', this.height);
    this.setStyle('minWidth', this.minWidth);
    this.setStyle('minHeight', this.minHeight);
    this.setStyle('maxWidth', this.maxWidth);
    this.setStyle('maxHeight', this.maxHeight);
    this.setStyle('flex', this.flex);
    this.setStyle('flexGrow', this.flexGrow);
    this.setStyle('flexShrink', this.flexShrink);
    this.setStyle('flexBasis', this.flexBasis);
    this.setStyle('flexDirection', this.flexDirection);
    this.setStyle('flexWrap', this.flexWrap);
    this.setStyle('justifyContent', this.justifyContent);
    this.setStyle('alignItems', this.alignItems);
    this.setStyle('alignSelf', this.alignSelf);
    this.setStyle('alignContent', this.alignContent);
    this.setStyle('gap', this.gap);
    this.setStyle('margin', this.margin);
    this.setStyle('padding', this.padding);
    this.setStyle('position', this.position);
    this.setStyle('top', this.top);
    this.setStyle('right', this.right);
    this.setStyle('bottom', this.bottom);
    this.setStyle('left', this.left);
    this.setStyle('opacity', this.opacity);
    this.setStyle('backgroundColor', this.backgroundColor);
    this.setStyle('borderWidth', this.borderWidth);
    this.setStyle('borderColor', this.borderColor);
    this.setStyle('borderRadius', this.borderRadius);
  }

  private setStyle(key: string, value: string | number | undefined): void {
    if (value === undefined || value === null) return;
    this.renderer.setStyle(this.element, key, String(value));
  }
}




