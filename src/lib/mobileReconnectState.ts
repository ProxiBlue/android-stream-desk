import type { ConnectionState } from '../types';

export interface ScanAgainCtaInput {
  isAndroidTauriApp: boolean;
  status: ConnectionState;
  reconnectAttempts: number;
  maxReconnectAttempts: number;
  pairingRequired?: boolean;
}

export function shouldShowScanAgainCta(input: ScanAgainCtaInput): boolean {
  if (!input.isAndroidTauriApp) return false;
  if (input.pairingRequired) return true;
  return input.status === 'error' && input.reconnectAttempts >= input.maxReconnectAttempts;
}

export function formatReconnectEndpoint(ipAddress: string, port: string): string {
  const host = ipAddress.trim();
  const wsPort = port.trim();
  if (!host || !wsPort) return '';
  return `${host}:${wsPort}`;
}
