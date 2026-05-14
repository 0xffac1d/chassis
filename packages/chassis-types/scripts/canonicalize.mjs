// RFC 8785-shaped JSON canonicalizer, matching Python's
//   json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
//
// Important subtleties vs JSON.stringify:
//   - keys are emitted in sorted order (recursively)
//   - no whitespace
//   - ensure_ascii=False: non-ASCII chars are emitted as raw UTF-8,
//     NOT as \u escapes. JSON.stringify already does raw UTF-8 for
//     non-ASCII since it returns a JS string (which node serializes
//     as UTF-8 on the wire). So we match Python by default there.
//   - control chars U+0000..U+001F are escaped, plus " and \.
//     Python escapes 0x00..0x1f using either the named escapes
//     (\b \t \n \f \r) or \u00XX. JSON.stringify does the same.
//   - U+2028 / U+2029: Python does NOT escape these with
//     ensure_ascii=False. JSON.stringify also does not escape them.
//     Consistent. Good.
//   - slash (/) is NOT escaped by Python. JSON.stringify does not
//     escape it either. Consistent.
//
// The remaining concern is number formatting. Both Python `json.dumps`
// and JS `JSON.stringify` emit the shortest round-tripping decimal
// for IEEE-754 doubles, which is identical on both platforms for the
// inputs we hash (JSON schemas contain small integers, no exotic
// floats). If a schema ever introduces a float like 1e+21 we'd need a
// custom number formatter; for now integer-only is the observed case.

function canonicalizeValue(value) {
  if (value === null) return 'null';
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) {
      // Python's json.dumps raises on NaN/Infinity by default; match that.
      throw new Error(`canonicalize: non-finite number ${value}`);
    }
    // JSON.stringify emits integers without a decimal point and uses
    // the shortest round-tripping form, same as Python json for the
    // schema inputs in chassis.
    return JSON.stringify(value);
  }
  if (typeof value === 'string') {
    // JSON.stringify already emits the same escape set as Python's
    // json.dumps(ensure_ascii=False):
    //   \" \\ \b \t \n \f \r \u00XX for remaining U+0000..U+001F
    //   and leaves all non-ASCII as raw UTF-8.
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    const parts = value.map(canonicalizeValue);
    return '[' + parts.join(',') + ']';
  }
  if (typeof value === 'object') {
    const keys = Object.keys(value).sort();
    const parts = [];
    for (const k of keys) {
      parts.push(JSON.stringify(k) + ':' + canonicalizeValue(value[k]));
    }
    return '{' + parts.join(',') + '}';
  }
  throw new Error(`canonicalize: unsupported type ${typeof value}`);
}

export function canonicalize(obj) {
  return canonicalizeValue(obj);
}
