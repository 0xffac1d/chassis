/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/dsse-envelope.schema.json
 */

/**
 * Dead Simple Signing Envelope (https://github.com/secure-systems-lab/dsse). Field names are camelCase by DSSE specification — this is the documented exception to the project-wide snake_case preference. Chassis writes payloadType = application/vnd.in-toto+json and a single Ed25519 signature.
 */
export interface DsseEnvelope {
  /**
   * Base64-encoded payload bytes (the in-toto Statement JSON, in Chassis's case).
   */
  payload: string;
  /**
   * Media type of the decoded payload. Chassis always emits 'application/vnd.in-toto+json'.
   */
  payloadType: string;
  /**
   * @minItems 1
   */
  signatures: [
    {
      /**
       * Optional key identifier (DSSE spec).
       */
      keyid?: string;
      /**
       * Base64-encoded signature over DSSE PAE.
       */
      sig: string;
    },
    ...{
      /**
       * Optional key identifier (DSSE spec).
       */
      keyid?: string;
      /**
       * Base64-encoded signature over DSSE PAE.
       */
      sig: string;
    }[]
  ];
}
