/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/in-toto-statement-v1.schema.json
 */

export interface InTotoStatementV1 {
  _type: 'https://in-toto.io/Statement/v1';
  /**
   * @minItems 1
   */
  subject: [
    {
      name: string;
      digest: {
        sha256: string;
      };
    },
    ...{
      name: string;
      digest: {
        sha256: string;
      };
    }[]
  ];
  predicateType: 'https://chassis.dev/attestation/release-gate/v1';
  predicate: {};
}
