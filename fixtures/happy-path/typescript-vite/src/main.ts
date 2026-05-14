export function greet(name: string): string {
  return `hello, ${name}`;
}

if (import.meta.hot) {
  // Hot-module reload sanity hook — exercises Vite detection heuristics.
}

console.log(greet("typescript-vite"));
