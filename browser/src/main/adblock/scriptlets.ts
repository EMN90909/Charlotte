/**
 * Safe subset of uBO-style scriptlets. Filter-list text is UNTRUSTED:
 * never eval scriptlet text or pass it to executeJavaScript verbatim.
 * Only two built-in, reviewed operations and restricted property paths
 * are accepted. Scripts run in page world only in Strict mode.
 */
export type SafeScriptlet =
  | { kind: 'set-constant'; property: string; value: 'undefined' | 'null' | 'false' | 'true' | '0' }
  | { kind: 'abort-on-property-read'; property: string };

const FORBIDDEN = new Set([
  '__proto__', 'prototype', 'constructor', 'window', 'globalThis',
  'document', 'location', 'navigator', 'top', 'parent', 'frames',
  'localStorage', 'sessionStorage', 'chrome', 'electron',
]);

export function parseSafeScriptlets(injections: readonly string[]): SafeScriptlet[] {
  const accepted: SafeScriptlet[] = [];
  const unique = new Set<string>();
  for (const injection of injections) {
    if (accepted.length >= 12 || injection.length > 180) break;
    const match = /^(set-constant|abort-on-property-read)\(([^()]*)\)$/.exec(injection.trim());
    if (!match) continue;
    const args = match[2].split(',').map((s) => s.trim());
    const property = args[0];
    if (!property || property.length > 80) continue;
    const segments = property.split('.');
    if (segments.length > 3 || segments.some((segment) =>
      !/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(segment) || FORBIDDEN.has(segment))) continue;
    if (match[1] === 'set-constant') {
      const value = args[1];
      if (args.length !== 2 || !['undefined', 'null', 'false', 'true', '0'].includes(value)) continue;
      const item: SafeScriptlet = { kind: 'set-constant', property, value: value as 'undefined' | 'null' | 'false' | 'true' | '0' };
      const key = JSON.stringify(item);
      if (!unique.has(key)) { unique.add(key); accepted.push(item); }
    } else {
      if (args.length !== 1) continue;
      const item: SafeScriptlet = { kind: 'abort-on-property-read', property };
      const key = JSON.stringify(item);
      if (!unique.has(key)) { unique.add(key); accepted.push(item); }
    }
  }
  return accepted;
}

/** Generate only our constant, audited function with JSON data, never JS from lists. */
export function compileSafeScriptletSource(injections: readonly string[]): string | null {
  const actions = parseSafeScriptlets(injections);
  if (!actions.length) return null;
  return `(() => {
    'use strict';
    const rules = ${JSON.stringify(actions)};
    const values = { undefined: undefined, null: null, false: false, true: true, 0: 0 };
    for (const rule of rules) {
      try {
        const path = rule.property.split('.');
        let target = window;
        for (let i = 0; i < path.length - 1; i++) {
          target = target[path[i]];
          if (!target || (typeof target !== 'object' && typeof target !== 'function')) break;
        }
        if (!target || (typeof target !== 'object' && typeof target !== 'function')) continue;
        const key = path[path.length - 1];
        const current = Object.getOwnPropertyDescriptor(target, key);
        if (current && !current.configurable) continue;
        if (rule.kind === 'set-constant') {
          Object.defineProperty(target, key, {
            configurable: true, enumerable: true,
            get: () => values[rule.value], set: () => {},
          });
        } else if (rule.kind === 'abort-on-property-read') {
          Object.defineProperty(target, key, {
            configurable: true, enumerable: true,
            get: () => { throw new ReferenceError('Blocked ad script access'); },
            set: () => {},
          });
        }
      } catch { /* Never stop other rules or page initialization. */ }
    }
  })();`;
}
