/**
 * AOT fixture for @angnative/web tests.
 *
 * This component is compiled by real Angular ngc into
 * `compiled/web-aot-fixture.component.js`. The compiled Ivy output is checked
 * in so `pnpm test` never needs the JIT compiler or generated dist artifacts.
 */
import { Component } from '@angular/core';
import { NaViewComponent, NaTextComponent, } from '@angnative/common';
import * as i0 from "@angular/core";
export class WebAotFixtureComponent {
    label = 'Hello';
    clicks = 0;
    onClick() {
        this.clicks++;
        this.label = `Clicked ${this.clicks}`;
    }
    static ɵfac = function WebAotFixtureComponent_Factory(__ngFactoryType__) { return new (__ngFactoryType__ || WebAotFixtureComponent)(); };
    static ɵcmp = /*@__PURE__*/ i0.ɵɵdefineComponent({ type: WebAotFixtureComponent, selectors: [["app-web-test"]], decls: 5, vars: 2, consts: [[3, "padding"], ["id", "tap", 3, "click"]], template: function WebAotFixtureComponent_Template(rf, ctx) { if (rf & 1) {
            i0.ɵɵelementStart(0, "na-view", 0)(1, "na-text");
            i0.ɵɵtext(2);
            i0.ɵɵelementEnd();
            i0.ɵɵelementStart(3, "div", 1);
            i0.ɵɵlistener("click", function WebAotFixtureComponent_Template_div_click_3_listener() { return ctx.onClick(); });
            i0.ɵɵtext(4, "Tap");
            i0.ɵɵelementEnd()();
        } if (rf & 2) {
            i0.ɵɵproperty("padding", "8px");
            i0.ɵɵadvance(2);
            i0.ɵɵtextInterpolate(ctx.label);
        } }, dependencies: [NaViewComponent, NaTextComponent], encapsulation: 2 });
}
(() => { (typeof ngDevMode === "undefined" || ngDevMode) && i0.ɵsetClassMetadata(WebAotFixtureComponent, [{
        type: Component,
        args: [{
                selector: 'app-web-test',
                standalone: true,
                imports: [NaViewComponent, NaTextComponent],
                template: `
    <na-view [padding]="'8px'">
      <na-text>{{ label }}</na-text>
      <div id="tap" (click)="onClick()">Tap</div>
    </na-view>
  `,
            }]
    }], null, null); })();
(() => { (typeof ngDevMode === "undefined" || ngDevMode) && i0.ɵsetClassDebugInfo(WebAotFixtureComponent, { className: "WebAotFixtureComponent", filePath: "web-aot-fixture.component.ts", lineNumber: 25 }); })();
