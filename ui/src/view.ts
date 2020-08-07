import { SvelteComponent } from "svelte";
import { Readable } from "svelte/store";

import * as history from "./history";

export interface View<K extends string> {
  readonly component: unknown;
  readonly key: K,
  readonly props?: Props;
}

export type Props = Record<string, 0 | string>;
export type Map<Key extends string, C extends typeof SvelteComponent> = Required<
  Record<Key, C>
>;

export interface Navigation<Key extends string> {
  readonly current: Readable<View<Key>>;
  back(): void;
  set(key: Key, props?: Props): void;
}

export const create = <K extends string, C extends typeof SvelteComponent>(
  componentMap: Map<K, C>,
  initial: K
): Navigation<K> => {
  const hist = history.create<View<K>>({ component: componentMap[initial], key: initial });

  return {
    current: hist.current,
    back: (): void => {
      hist.pop();
    },
    set: (key: K, props?: Props): void => {
      hist.push({
        component: componentMap[key],
        key,
        props,
      });
    },
  };
};

export interface MenuItem<K extends string> {
  icon: typeof SvelteComponent,
  key: K,
  title: string,
}
