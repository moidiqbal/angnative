/**
 * MarketPulse AOT crypto demo — heavy windowed OHLC rows + Angular Signals +
 * an RxJS quote stream + the `na-candlestick` contract element.
 *
 * BTC/USD data comes only from the Android host's Coinbase Exchange WebSocket
 * `matches` stream. Kotlin builds 30-minute OHLC bars from executed match
 * prices and sizes; this component never fetches a REST market endpoint and
 * never fabricates history or volume. A real RxJS `interval` asks the host
 * (through the async HttpClient/fetch shim) for the latest socket-derived
 * quote every 30s; the status reports live/last-good honestly and makes no
 * real-time guarantee.
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
  computed,
  OnDestroy,
  OnInit,
  provideZonelessChangeDetection,
  signal,
  type ApplicationRef,
} from '@angular/core';
import { bootstrapApplication } from '@angular/platform-browser';
import {
  NaCandlestickComponent,
  NaPressableComponent,
  NaTextComponent,
  NaViewComponent,
} from '@angnative/common';
import { provideNativeAngular, provideNativeAnimations } from '@angnative/platform';
import { interval, Subject, type Subscription } from 'rxjs';

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
  fresh?: boolean;
  streamState?: string;
  error?: string;
}

interface DatasetRow {
  i: number;
  label: string;
}

const EMPTY_QUOTE_JSON =
  '{"symbol":"","feed":"Coinbase Exchange WebSocket · matches · 30m bars from executed matches · volume from match sizes","fetchedAt":"","fresh":false,"streamState":"connecting","bars":[]}';

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
        [text]="status()"
      ></na-text>
      <div
        [@flash]="flashState()"
        (@flash.done)="onFlashDone()"
        [style.width]="'340px'"
        [style.height]="'30px'"
      >
        <na-text
          [style.width]="'330px'"
          [style.height]="'30px'"
          [style.fontSize]="'18px'"
          [text]="symbol() + ' ' + lastPrice()"
        ></na-text>
      </div>
      <na-candlestick
        [attr.bars]="quoteJson()"
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
export class MarketPulseAppComponent implements OnInit, OnDestroy {
  /** Deterministic 10,000-row dataset; only the window below reaches native views. */
  readonly datasetSize = 10000;
  readonly windowSize = 200;
  readonly rows: DatasetRow[] = Array.from({ length: this.datasetSize }, (_, i) => ({
    i,
    label: `OHLC ${i} · seed42`,
  }));

  visibleRows: DatasetRow[] = this.rows.slice(0, this.windowSize);

  private readonly quote = signal<QuotePayload | null>(null);
  readonly status = signal(
    'waiting for Coinbase Exchange WebSocket matches · 30m bars (not guaranteed real-time)',
  );
  readonly flashState = signal('idle');
  readonly streamEvents = signal(0);

  readonly symbol = computed(() => this.quote()?.symbol || '—');
  readonly barCount = computed(() => this.quote()?.bars.length ?? 0);
  readonly lastPrice = computed(() => {
    const quote = this.quote();
    return quote && quote.bars.length > 0 ? String(quote.bars[quote.bars.length - 1]!.c) : '';
  });
  readonly quoteJson = computed(() => {
    const quote = this.quote();
    return quote ? JSON.stringify(quote) : EMPTY_QUOTE_JSON;
  });

  events: string[] = [];

  /** RxJS stream: initial fetch, real 30s refresh, socket push and fixture. */
  private readonly quoteInput$ = new Subject<string>();
  private readonly streamSubscription: Subscription;
  private readonly pollSubscription: Subscription;
  // HttpClient returns Angular's nested RxJS 7 Subscription while this bundle
  // imports the workspace RxJS 6; keep only the structural part we use.
  private refreshSubscription: { unsubscribe(): void } | null = null;
  private refreshInFlight = false;
  private static readonly REFRESH_INTERVAL_MS = 30_000;

  constructor(
    private readonly http: HttpClient,
    private readonly cdr: ChangeDetectorRef,
  ) {
    this.streamSubscription = this.quoteInput$.subscribe({
      next: (json) => {
        try {
          const quote = JSON.parse(json) as QuotePayload;
          if (!quote || !Array.isArray(quote.bars)) throw new Error('invalid quote payload');
          this.streamEvents.update((count) => count + 1);
          this.applyQuote(quote);
        } catch (err) {
          this.fail(String((err as { message?: string })?.message ?? err));
        }
      },
      error: (err: unknown) => this.fail(String((err as { message?: string })?.message ?? err)),
    });
    // Real RxJS interval (host-backed timer via na_timerSchedule). It does not
    // fetch market data itself; it asks the host for the latest socket payload.
    this.pollSubscription = interval(MarketPulseAppComponent.REFRESH_INTERVAL_MS).subscribe(() =>
      this.refreshQuote(),
    );
  }

  ngOnInit(): void {
    // Keep the initial fetch; it is served by the async fetch shim from the
    // host's socket-derived quote, not by a REST market endpoint.
    this.refreshQuote();
    this.flush();
  }

  ngOnDestroy(): void {
    this.pollSubscription.unsubscribe();
    this.refreshSubscription?.unsubscribe();
    this.streamSubscription.unsubscribe();
  }

  /** Push a normalized JSON payload into the RxJS stream. */
  pushQuote(json: string): void {
    this.quoteInput$.next(json);
  }

  /**
   * One in-flight HttpClient request at a time. The request is routed through
   * the async fetch shim to the host's cached Coinbase WebSocket payload.
   */
  refreshQuote(): void {
    if (this.refreshInFlight) return;
    this.refreshInFlight = true;
    this.refreshSubscription?.unsubscribe();
    this.refreshSubscription = this.http.get<QuotePayload>('/marketpulse/quote').subscribe({
      next: (body) => {
        this.refreshInFlight = false;
        this.quoteInput$.next(JSON.stringify(body));
      },
      error: (err: unknown) => {
        this.refreshInFlight = false;
        this.fail(String((err as { message?: string })?.message ?? err));
      },
    });
  }

  applyQuote(quote: QuotePayload): void {
    this.quote.set(quote);
    const count = quote.bars.length;
    const feed = quote.feed || 'public OHLC feed';
    const state = quote.streamState || 'connecting';
    const hasStreamMeta = quote.fresh !== undefined || quote.streamState !== undefined;
    let status: string;
    if (quote.error) {
      status = `feed unavailable: ${quote.error} · ${count} last-good 30m bars · ${state} · ${feed}`;
    } else if (!hasStreamMeta) {
      // Deterministic fixture path (no socket metadata): keep the legacy shape.
      status = `${quote.symbol} · ${count} bars · ${feed}`;
    } else {
      const mode = quote.fresh === true ? 'live' : count > 0 ? 'last-good' : 'waiting';
      status =
        `${quote.symbol} · ${count} 30m match-derived bars · ${mode} · ${state} · ${feed}` +
        ' · not guaranteed real-time';
    }
    this.status.set(status);
    this.flashState.set('flash');
    this.flush();
  }

  fail(reason: string): void {
    // Keep the last good payload visible; never fabricate market data.
    this.status.set(`feed unavailable: ${reason} · last-good data kept`);
    this.flush();
  }

  onRefreshClick(): void {
    this.status.set('refresh requested from Coinbase Exchange WebSocket · latest cached quote');
    this.refreshQuote();
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
    this.pushQuote(
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
    this.flashState.set(value);
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
  var __marketPulseStreamEvents: (() => number) | undefined;
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
    globalThis.__marketPulseApplyQuote = (json: string) => app.pushQuote(json);
    globalThis.__marketPulseQuote = () => app.quoteJson();
    globalThis.__marketPulseStatus = () => app.status();
    globalThis.__marketPulseRowCount = () => app.visibleRows.length;
    globalThis.__marketPulseStreamEvents = () => app.streamEvents();
    globalThis.__setMarketPulseAnimationState = (value: string) => app.setState(value);
    globalThis.__marketPulseAnimationEvents = () => app.events.slice();
  })
  .catch((err: unknown) => {
    const error = err as { message?: string; stack?: string };
    globalThis.__marketPulseError = String(error.stack ?? error.message ?? err);
  });
