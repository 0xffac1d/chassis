/**
 * Minimal consumer: proves @chassis/types resolves and declarations are usable.
 */
import type { Contract } from '@chassis/types';

/** Concrete slice of the kind-discriminated contract union. */
export type LibraryContract = Extract<Contract, { kind: 'library' }>;

export const demoLibraryContract: LibraryContract = {
  kind: 'library',
  name: 'consumer-fixture',
  purpose: 'Keeps the chassis-types consumer typecheck exercising a concrete contract kind.',
  status: 'draft',
  since: '0.1.0',
  version: '0.1.0',
  assurance_level: 'declared',
  owner: 'chassis-types-fixtures',
  exports: [],
  invariants: [{ id: 'consumer.types-resolve', text: 'This file compiles against generated Contract typings.' }],
  edge_cases: [],
};
