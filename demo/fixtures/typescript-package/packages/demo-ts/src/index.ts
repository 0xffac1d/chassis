// @claim demo.ts.greeting
// @claim demo.ts.empty-name
export function greeting(name: string): string {
  const trimmed = name.trim();
  const who = trimmed.length === 0 ? "friend" : trimmed;
  return `hello, ${who}`;
}
