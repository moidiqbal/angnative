/**
 * MarketPulse AOT crypto demo — heavy windowed OHLC rows + real Angular
 * animations + the `na-candlestick` contract element.
 *
 * BTC/USD OHLC data is fetched by the Android host from CoinGecko's public
 * endpoint and delivered through the synchronous fetch handler /
 * __marketPulseApplyQuote hook. Bars are 30-minute delayed OHLC; no
 * real-time claim or fabricated data is made here.
 */
import {
  animate,
  keyframes,
  state,
  style,
  transition,
  trigger,
} from '@angular/animations';
import { HttpClient, provideHttpClient } from '@angular/common/http';
import {
  ChangeDetectorRef,
  Component,
  provideZonelessChangeDetection,
  type ApplicationRef,
  type OnInit,
} from '@angular/core';
import { bootstrapApplication } from '@angular/platform-browser';
import {
  NaCandlestickComponent,
  NaPressableComponent,
  NaTextComponent,
  NaViewComponent,
} from '@angnative/common';
import { provideNativeAngular, provideNativeAnimations } from '@angnative/platform';

interface QuoteBar {
  t: number;
  o: number;
  h: number;
  l: number;
  c: number;
  v: number;
}

interface QuotePayload {
  symbol: string;
  feed: string;
  fetchedAt: string;
  bars: QuoteBar[];
  error?: string;
}

interface DatasetRow {
  i: number;
  label: string;
}

@Component({
  selector: 'market-pulse-app',
  standalone: true,
  imports: [NaViewComponent, NaTextComponent, NaPressableComponent, NaCandlestickComponent],
  animations: [
    trigger('flash', [
      state('idle', style({ opacity: 1 })),
      state('flash', style({ opacity: 0.55 })),
      transition('idle => flash', [
        animate(
          '100ms',
          keyframes([
            style({ opacity: 1, offset: 0 }),
            style({ opacity: 0.5, offset: 0.5 }),
            style({ opacity: 0.55, offset: 1 }),
          ]),
        ),
      ]),
    ]),
  ],
  template: `
    <na-view
      [style.flexDirection]="'column'"
      [style.width]="'360px'"
      [style.height]="'3600px'"
      [style.padding]="'12px'"
      [style.gap]="'8px'"
    >
      <na-text
        [style.width]="'340px'"
        [style.height]="'30px'"
        [style.fontSize]="'18px'"
        [style.fontWeight]="'bold'"
        [text]="'MarketPulse — BTC/USD OHLC'"
      ></na-text>
      <na-text
        [style.width]="'340px'"
        [style.height]="'36px'"
        [style.fontSize]="'13px'"
        [text]="status"
      ></na-text>
      <div
        [@flash]="flashState"
        (@flash.done)="onFlashDone()"
        [style.width]="'340px'"
        [style.height]="'30px'"
      >
        <na-text
          [style.width]="'330px'"
          [style.height]="'30px'"
          [style.fontSize]="'18px'"
          [text]="symbol + ' ' + lastPrice"
        ></na-text>
      </div>
      <na-candlestick
        [attr.bars]="quoteJson"
        [style.width]="'340px'"
        [style.height]="'240px'"
      ></na-candlestick>
      <div
        [style.flexDirection]="'row'"
        [style.width]="'340px'"
        [style.height]="'42px'"
        [style.gap]="'12px'"
      >
        <na-pressable
          data-action="refresh"
          (click)="onRefreshClick()"
          [style.width]="'160px'"
          [style.height]="'42px'"
        >
          <na-text
            [style.width]="'140px'"
            [style.height]="'30px'"
            [style.fontSize]="'15px'"
            [text]="'Refresh'"
          ></na-text>
        </na-pressable>
        <na-pressable
          data-action="demo-bars"
          (press)="onDemoBarsClick()"
          [style.width]="'160px'"
          [style.height]="'42px'"
        >
          <na-text
            [style.width]="'140px'"
            [style.height]="'30px'"
            [style.fontSize]="'15px'"
            [text]="'Demo bars'"
          ></na-text>
        </na-pressable>
      </div>
      @for (row of visibleRows; track row.i) {
        <na-text
          [style.width]="'320px'"
          [style.height]="'22px'"
          [style.fontSize]="'14px'"
          [text]="row.label"
        ></na-text>
      }
    </na-view>
  `,
})
export class MarketPulseAppComponent implements OnInit {
  /** Deterministic 10,000-row dataset; only the window below reaches native views. */
  readonly datasetSize = 10000;
  readonly windowSize = 200;
  readonly rows: DatasetRow[] = Array.from({ length: this.datasetSize }, (_, i) => ({
    i,
    label: `OHLC ${i} · seed42`,
  }));

  visibleRows: DatasetRow[] = this.rows.slice(0, this.windowSize);
  symbol = '—';
  lastPrice = '';
  status = 'waiting for BTC/USD OHLC quote';
  quoteJson =
    '{"symbol":"","feed":"CoinGecko public OHLC · 30m bars","fetchedAt":"","bars":[]}';
  barCount = 0;
  flashState = 'idle';
  events: string[] = [];

  constructor(
    private readonly http: HttpClient,
    private readonly cdr: ChangeDetectorRef,
  ) {}

  ngOnInit(): void {
    this.http.get<QuotePayload>('/marketpulse/quote').subscribe({
      next: (body) => this.applyQuote(JSON.stringify(body)),
      error: (err: unknown) => this.fail(String((err as { message?: string })?.message ?? err)),
    });
    this.flush();
  }

  applyQuote(json: string): void {
    try {
      const quote = JSON.parse(json) as QuotePayload;
      if (!Array.isArray(quote.bars)) throw new Error('missing bars array');
      this.symbol = quote.symbol || '—';
      this.barCount = quote.bars.length;
      this.lastPrice = this.barCount > 0 ? String(quote.bars[this.barCount - 1]!.c) : '';
      this.quoteJson = json;
      const feed = quote.feed || 'public OHLC feed';
      this.status = quote.error
        ? `feed unavailable: ${quote.error} · ${this.barCount} last-good bars`
        : `${this.symbol} · ${this.barCount} bars · ${feed}`;
      this.flashState = 'flash';
    } catch (err) {
      this.fail(String((err as { message?: string })?.message ?? err));
    }
    this.flush();
  }

  fail(reason: string): void {
    // Keep the last good payload visible; never fabricate market data.
    this.status = `feed unavailable: ${reason}`;
    this.flush();
  }

  onRefreshClick(): void {
    this.status = 'refresh requested from BTC/USD OHLC feed';
    this.flush();
  }

  /** Explicit demo path: deterministic BTC-like bars, clearly labelled as not live. */
  onDemoBarsClick(): void {
    const bars = Array.from({ length: 32 }, (_, i) => ({
      t: 1_795_530_600_000 + i * 1_800_000,
      o: 60_000 + i * 25,
      h: 60_150 + i * 25 + (i % 3) * 40,
      l: 59_850 + i * 25 - (i % 2) * 30,
      c: 60_075 + i * 25 + (i % 4) * 15,
      v: 0,
    }));
    this.applyQuote(
      JSON.stringify({
        symbol: 'BTC/USD',
        feed: 'deterministic BTC/USD fixture (not live)',
        fetchedAt: '2026-11-25T00:00:00Z',
        bars,
      }),
    );
  }

  onFlashDone(): void {
    this.events.push('done');
  }

  setState(value: string): void {
    this.flashState = value;
    this.flush();
  }

  flush(): void {
    this.cdr.detectChanges();
  }
}

declare global {
  // eslint-disable-next-line no-var
  var __marketPulseError: string | undefined;
  // eslint-disable-next-line no-var
  var __marketPulseApplyQuote: ((json: string) => void) | undefined;
  // eslint-disable-next-line no-var
  var __marketPulseQuote: (() => string) | undefined;
  // eslint-disable-next-line no-var
  var __marketPulseStatus: (() => string) | undefined;
  // eslint-disable-next-line no-var
  var __marketPulseRowCount: (() => number) | undefined;
  // eslint-disable-next-line no-var
  var __setMarketPulseAnimationState: ((state: string) => void) | undefined;
  // eslint-disable-next-line no-var
  var __marketPulseAnimationEvents: (() => string[]) | undefined;
}

bootstrapApplication(MarketPulseAppComponent, {
  providers: [
    provideZonelessChangeDetection(),
    provideNativeAngular(),
    provideNativeAnimations(),
    provideHttpClient(),
  ],
})
  .then((ref: ApplicationRef) => {
    const app = ref.components[0]!.instance as MarketPulseAppComponent;
    globalThis.__marketPulseApplyQuote = (json: string) => app.applyQuote(json);
    globalThis.__marketPulseQuote = () => app.quoteJson;
    globalThis.__marketPulseStatus = () => app.status;
    globalThis.__marketPulseRowCount = () => app.visibleRows.length;
    globalThis.__setMarketPulseAnimationState = (value: string) => app.setState(value);
    globalThis.__marketPulseAnimationEvents = () => app.events.slice();
  })
  .catch((err: unknown) => {
    const error = err as { message?: string; stack?: string };
    globalThis.__marketPulseError = String(error.stack ?? error.message ?? err);
  });
