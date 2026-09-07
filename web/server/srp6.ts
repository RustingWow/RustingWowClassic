import { createHash, randomBytes } from "node:crypto";

/** WoW SRP6 generator `g`. */
export const GENERATOR = 7n;

/** Large safe prime `N` as little-endian bytes (wow_srp). */
export const LARGE_SAFE_PRIME_LITTLE_ENDIAN = Buffer.from([
  0xb7, 0x9b, 0x3e, 0x2a, 0x87, 0x82, 0x3c, 0xab, 0x8f, 0x5e, 0xbf, 0xbf, 0x8e,
  0xb1, 0x01, 0x08, 0x53, 0x50, 0x06, 0x29, 0x8b, 0x5b, 0xad, 0xbd, 0x5b, 0x53,
  0xe1, 0x89, 0x5e, 0x64, 0x4b, 0x89,
]);

const N = bytesToBigIntLE(LARGE_SAFE_PRIME_LITTLE_ENDIAN);

export function normalizeUsername(username: string): string | null {
  const normalized = username.trim().toUpperCase();
  if (normalized.length < 2 || normalized.length > 16) {
    return null;
  }
  if (!/^[A-Z0-9]+$/.test(normalized)) {
    return null;
  }
  return normalized;
}

export function generateSalt(): Buffer {
  return randomBytes(32);
}

export function calculateVerifier(
  username: string,
  password: string,
  salt: Buffer,
): Buffer {
  const user = username.toUpperCase();
  const pass = password.toUpperCase();
  const inner = sha1(Buffer.from(`${user}:${pass}`, "ascii"));
  const xHash = sha1(Buffer.concat([salt, inner]));
  const x = bytesToBigIntLE(xHash);
  const verifier = modPow(GENERATOR, x, N);
  return bigIntToBytesLE(verifier, 32);
}

function sha1(data: Buffer): Buffer {
  return createHash("sha1").update(data).digest();
}

function bytesToBigIntLE(bytes: Buffer): bigint {
  let value = 0n;
  for (let i = bytes.length - 1; i >= 0; i -= 1) {
    value = (value << 8n) + BigInt(bytes[i]);
  }
  return value;
}

function bigIntToBytesLE(value: bigint, length: number): Buffer {
  const bytes = Buffer.alloc(length);
  let remaining = value;
  for (let i = 0; i < length; i += 1) {
    bytes[i] = Number(remaining & 0xffn);
    remaining >>= 8n;
  }
  return bytes;
}

function modPow(base: bigint, exponent: bigint, modulus: bigint): bigint {
  let result = 1n;
  let b = base % modulus;
  let e = exponent;
  while (e > 0n) {
    if (e & 1n) {
      result = (result * b) % modulus;
    }
    b = (b * b) % modulus;
    e >>= 1n;
  }
  return result;
}
