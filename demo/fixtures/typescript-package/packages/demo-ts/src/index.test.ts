import { greeting } from "./index";

function test(_name: string, run: () => void): void {
  run();
}

function expectEqual(actual: string, expected: string): void {
  if (actual !== expected) {
    throw new Error(`expected ${expected}, got ${actual}`);
  }
}

// @claim demo.ts.greeting
test("returns stable greeting", () => {
  expectEqual(greeting("Ada"), "hello, Ada");
});

// @claim demo.ts.empty-name
test("defaults blank name", () => {
  expectEqual(greeting("  "), "hello, friend");
});
