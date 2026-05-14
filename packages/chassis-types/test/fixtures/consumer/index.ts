/**
 * Minimal consumer: proves @chassis/types resolves and declarations are usable.
 */
import type { AgentAction } from '@chassis/types';
import type { Agent_AgentAction } from '@chassis/types';

export type DemoBare = AgentAction;
export type DemoNamespaced = Agent_AgentAction.AgentAction;
