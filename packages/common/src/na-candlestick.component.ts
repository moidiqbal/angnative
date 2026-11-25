/**
 * NaCandlestickComponent — native OHLC candlestick chart (`<na-candlestick>`).
 *
 * Maps to the canonical `na-candlestick` contract / `Custom:na-candlestick`
 * wire node. The chart data arrives as a JSON **string** through the
 * `attr:bars` attribute; native shells parse it and draw with Canvas.
 */

import {
  ChangeDetectionStrategy,
  Component,
} from '@angular/core';

@Component({
  selector: 'na-candlestick',
  standalone: true,
  template: ``,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NaCandlestickComponent {}
