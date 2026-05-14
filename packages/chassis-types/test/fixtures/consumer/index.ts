/**
 * Minimal consumer: proves @chassis/types resolves and declarations are usable.
 * Canary file — if dist is rebuilt against a schema set that drops Contract,
 * this fails before publish.
 */
import type { Contract } from '@chassis/types';
import type { Contract_Contract } from '@chassis/types';

export type DemoBare = Contract;
export type DemoNamespaced = Contract_Contract.Contract;
