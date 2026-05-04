import { describe, expect, it } from 'vitest';
import {
  getNotificationChannelReadiness,
  parseWhatsAppEventPreferences,
  sanitizeWhatsAppEventPreference,
  settingsToWhatsAppGatewayForm,
  whatsappGatewayFormToSettings,
} from './whatsappGateway';
import type { WhatsAppEventDefinition, WhatsAppGatewayFormState } from '$lib/api/types';

const events: WhatsAppEventDefinition[] = [
  { code: 'customer_invoice_due', label: 'Invoice due', scope: 'tenant' },
  { code: 'network_router_down', label: 'Router down', scope: 'tenant' },
];

describe('whatsapp gateway settings helpers', () => {
  it('converts disabled settings to disabled form state', () => {
    expect(settingsToWhatsAppGatewayForm({})).toMatchObject({
      enabled: false,
      provider: 'disabled',
      fonnteToken: '',
      triwaxApiKey: '',
    });
  });

  it('converts Fonnte settings to form state', () => {
    expect(
      settingsToWhatsAppGatewayForm({
        wa_gateway_enabled: 'true',
        wa_gateway_provider: 'fonnte',
        wa_gateway_fonnte_token: 'secret-token',
        wa_gateway_fonnte_base_url: 'https://api.fonnte.com',
        wa_gateway_fonnte_sender: '62810000000',
      }),
    ).toMatchObject({
      enabled: true,
      provider: 'fonnte',
      fonnteToken: 'secret-token',
      fonnteBaseUrl: 'https://api.fonnte.com',
      fonnteSender: '62810000000',
    });
  });

  it('converts Triwax settings to form state', () => {
    expect(
      settingsToWhatsAppGatewayForm({
        wa_gateway_enabled: 'true',
        wa_gateway_provider: 'triwax',
        wa_gateway_triwax_api_key: 'triwax-key',
      }),
    ).toMatchObject({
      enabled: true,
      provider: 'triwax',
      triwaxApiKey: 'triwax-key',
    });
  });

  it('serializes form state to the WhatsApp setting keys', () => {
    const form: WhatsAppGatewayFormState = {
      enabled: true,
      provider: 'triwax',
      fonnteToken: 'secret-token',
      fonnteBaseUrl: 'https://api.fonnte.com',
      fonnteSender: '62810000000',
      triwaxApiKey: 'triwax-key',
    };

    expect(whatsappGatewayFormToSettings(form)).toEqual({
      wa_gateway_enabled: 'true',
      wa_gateway_provider: 'triwax',
      wa_gateway_fonnte_token: 'secret-token',
      wa_gateway_fonnte_base_url: 'https://api.fonnte.com',
      wa_gateway_fonnte_sender: '62810000000',
      wa_gateway_triwax_api_key: 'triwax-key',
    });
  });

  it('defaults missing event WhatsApp preferences to false', () => {
    expect(
      parseWhatsAppEventPreferences(
        JSON.stringify({
          customer_invoice_due: { email: true, in_app: true },
        }),
        events,
      ),
    ).toEqual({
      customer_invoice_due: { whatsapp: false, email: true, in_app: true },
      network_router_down: { whatsapp: false, email: false, in_app: false },
    });
  });

  it('ignores event codes outside the known registry', () => {
    expect(
      parseWhatsAppEventPreferences(
        JSON.stringify({
          unknown_event: { whatsapp: true, email: true, in_app: true },
        }),
        events,
      ),
    ).toEqual({
      customer_invoice_due: { whatsapp: false, email: false, in_app: false },
      network_router_down: { whatsapp: false, email: false, in_app: false },
    });
  });

  it('marks WhatsApp ready only when the selected gateway has required config', () => {
    expect(
      getNotificationChannelReadiness({
        wa_gateway_enabled: 'true',
        wa_gateway_provider: 'fonnte',
        wa_gateway_fonnte_token: 'secret',
      }).whatsapp.ready,
    ).toBe(true);

    expect(
      getNotificationChannelReadiness({
        wa_gateway_enabled: 'true',
        wa_gateway_provider: 'fonnte',
      }).whatsapp.ready,
    ).toBe(false);

    expect(
      getNotificationChannelReadiness({
        wa_gateway_enabled: 'true',
        wa_gateway_provider: 'triwax',
        wa_gateway_triwax_api_key: 'triwax-key',
      }).whatsapp.ready,
    ).toBe(true);
  });

  it('marks email ready from SMTP or API provider requirements', () => {
    expect(
      getNotificationChannelReadiness({
        email_provider: 'smtp',
        email_from_address: 'billing@example.com',
        email_smtp_host: 'smtp.example.com',
        email_smtp_port: '587',
        email_smtp_username: 'user',
        email_smtp_password: 'pass',
      }).email.ready,
    ).toBe(true);

    expect(
      getNotificationChannelReadiness({
        email_provider: 'smtp',
        email_from_address: 'billing@example.com',
        email_smtp_host: 'smtp.example.com',
      }).email.ready,
    ).toBe(false);

    expect(
      getNotificationChannelReadiness({
        email_provider: 'sendgrid',
        email_from_address: 'billing@example.com',
        email_api_key: 'key',
      }).email.ready,
    ).toBe(true);
  });

  it('sanitizes event preferences for unavailable external channels', () => {
    expect(
      sanitizeWhatsAppEventPreference(
        { whatsapp: true, email: true, in_app: true },
        {
          whatsapp: { ready: false, reason: 'Configure WhatsApp first' },
          email: { ready: false, reason: 'Configure email first' },
          in_app: { ready: true, reason: null },
        },
      ),
    ).toEqual({ whatsapp: false, email: false, in_app: true });
  });
});
