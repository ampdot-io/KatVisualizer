/* tslint:disable */
/* eslint-disable */

export class Visualizer {
  free(): void;
  [Symbol.dispose](): void;
  set_colors(left_hue: number, right_hue: number): void;
  process_mono(samples: Float32Array): void;
  process_stereo(left: Float32Array, right: Float32Array): void;
  set_canvas_size(width: number, height: number): void;
  get_peak_amplitude(): number;
  get_peak_frequency(): number;
  set_amplitude_range(min_db: number, max_db: number): void;
  set_bargraph_height(ratio: number): void;
  constructor(sample_rate: number, resolution: number);
  reset(): void;
  render(ctx: CanvasRenderingContext2D): void;
  get_pan(): number;
}

export function main(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_visualizer_free: (a: number, b: number) => void;
  readonly visualizer_get_pan: (a: number) => number;
  readonly visualizer_get_peak_amplitude: (a: number) => number;
  readonly visualizer_get_peak_frequency: (a: number) => number;
  readonly visualizer_new: (a: number, b: number) => number;
  readonly visualizer_process_mono: (a: number, b: number, c: number) => void;
  readonly visualizer_process_stereo: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly visualizer_render: (a: number, b: any) => [number, number];
  readonly visualizer_reset: (a: number) => void;
  readonly visualizer_set_amplitude_range: (a: number, b: number, c: number) => void;
  readonly visualizer_set_bargraph_height: (a: number, b: number) => void;
  readonly visualizer_set_canvas_size: (a: number, b: number, c: number) => void;
  readonly visualizer_set_colors: (a: number, b: number, c: number) => void;
  readonly main: () => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
