import { randomBytes } from 'crypto';

export function generateRandomHex(size: number): string {
  const bytes = randomBytes(size);
  return '0x' + bytes.toString('hex');
}

export function hexToBigInt(hexString: string): bigint {
  const cleanHex = hexString.startsWith('0x') ? hexString.slice(2) : hexString;
  return BigInt('0x' + cleanHex);
}