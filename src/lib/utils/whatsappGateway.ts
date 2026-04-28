import type {
  WhatsAppEventDefinition,
  WhatsAppEventPreference,
  WhatsAppGatewayFormState,
  WhatsAppGatewayProvider,
  WhatsAppGatewaySettingsMap,
} from '$lib/api/types';

export const WHATSAPP_GATEWAY_SETTING_KEYS = [
  'wa_gateway_enabled',
  'wa_gateway_provider',
  'wa_gateway_fonnte_token',
  'wa_gateway_fonnte_base_url',
  'wa_gateway_fonnte_sender',
  'wa_gateway_custom_url',
  'wa_gateway_custom_method',
  'wa_gateway_custom_headers',
  'wa_gateway_custom_body_template',
  'wa_gateway_custom_success_statuses',
] as const;

export const DEFAULT_WHATSAPP_GATEWAY_FORM: WhatsAppGatewayFormState = {
  enabled: false,
  provider: 'disabled',
  fonnteToken: '',
  fonnteBaseUrl: '',
  fonnteSender: '',
  customUrl: '',
  customMethod: 'POST',
  customHeaders: '',
  customBodyTemplate: '',
  customSuccessStatuses: '200,201,202',
};

export type NotificationChannelReadiness = {
  whatsapp: { ready: boolean; reason: string | null };
  email: { ready: boolean; reason: string | null };
  in_app: { ready: true; reason: null };
};

function readString(settings: WhatsAppGatewaySettingsMap, key: string): string {
  return `${settings[key] ?? ''}`;
}

function readBoolean(settings: WhatsAppGatewaySettingsMap, key: string): boolean {
  return readString(settings, key).trim().toLowerCase() === 'true';
}

function readProvider(settings: WhatsAppGatewaySettingsMap): WhatsAppGatewayProvider {
  const provider = readString(settings, 'wa_gateway_provider');
  if (provider === 'fonnte' || provider === 'custom_http') return provider;
  return 'disabled';
}

export function settingsToWhatsAppGatewayForm(
  settings: WhatsAppGatewaySettingsMap,
): WhatsAppGatewayFormState {
  const provider = readProvider(settings);

  return {
    ...DEFAULT_WHATSAPP_GATEWAY_FORM,
    enabled: provider !== 'disabled' && readBoolean(settings, 'wa_gateway_enabled'),
    provider,
    fonnteToken: readString(settings, 'wa_gateway_fonnte_token'),
    fonnteBaseUrl: readString(settings, 'wa_gateway_fonnte_base_url'),
    fonnteSender: readString(settings, 'wa_gateway_fonnte_sender'),
    customUrl: readString(settings, 'wa_gateway_custom_url'),
    customMethod: readString(settings, 'wa_gateway_custom_method') || 'POST',
    customHeaders: readString(settings, 'wa_gateway_custom_headers'),
    customBodyTemplate: readString(settings, 'wa_gateway_custom_body_template'),
    customSuccessStatuses:
      readString(settings, 'wa_gateway_custom_success_statuses') || '200,201,202',
  };
}

export function whatsappGatewayFormToSettings(
  form: WhatsAppGatewayFormState,
): Record<(typeof WHATSAPP_GATEWAY_SETTING_KEYS)[number], string> {
  return {
    wa_gateway_enabled: form.enabled ? 'true' : 'false',
    wa_gateway_provider: form.provider,
    wa_gateway_fonnte_token: form.fonnteToken,
    wa_gateway_fonnte_base_url: form.fonnteBaseUrl,
    wa_gateway_fonnte_sender: form.fonnteSender,
    wa_gateway_custom_url: form.customUrl,
    wa_gateway_custom_method: form.customMethod,
    wa_gateway_custom_headers: form.customHeaders,
    wa_gateway_custom_body_template: form.customBodyTemplate,
    wa_gateway_custom_success_statuses: form.customSuccessStatuses,
  };
}

function toPreference(value: unknown): WhatsAppEventPreference {
  if (!value || typeof value !== 'object') {
    return { whatsapp: false, email: false, in_app: false };
  }

  const row = value as Partial<WhatsAppEventPreference>;
  return {
    whatsapp: row.whatsapp === true,
    email: row.email === true,
    in_app: row.in_app === true,
  };
}

export function parseWhatsAppEventPreferences(
  json: string | null | undefined,
  events: WhatsAppEventDefinition[],
): Record<string, WhatsAppEventPreference> {
  let parsed: unknown = {};

  if (json && json.trim()) {
    try {
      parsed = JSON.parse(json);
    } catch {
      parsed = {};
    }
  }

  const source = parsed && typeof parsed === 'object' ? (parsed as Record<string, unknown>) : {};

  return Object.fromEntries(events.map((event) => [event.code, toPreference(source[event.code])]));
}

export function serializeWhatsAppEventPreferences(
  preferences: Record<string, WhatsAppEventPreference>,
): string {
  return JSON.stringify(preferences);
}

function hasValue(settings: WhatsAppGatewaySettingsMap, key: string): boolean {
  return readString(settings, key).trim().length > 0;
}

function getWhatsAppReadiness(settings: WhatsAppGatewaySettingsMap) {
  const provider = readProvider(settings);
  const enabled = provider !== 'disabled' && readBoolean(settings, 'wa_gateway_enabled');

  if (!enabled) {
    return { ready: false, reason: 'Enable and configure WhatsApp gateway first.' };
  }
  if (provider === 'fonnte' && !hasValue(settings, 'wa_gateway_fonnte_token')) {
    return { ready: false, reason: 'Add a Fonnte API token first.' };
  }
  if (provider === 'custom_http' && !hasValue(settings, 'wa_gateway_custom_url')) {
    return { ready: false, reason: 'Add a custom WhatsApp endpoint URL first.' };
  }

  return { ready: true, reason: null };
}

function getEmailReadiness(settings: WhatsAppGatewaySettingsMap) {
  const provider = (readString(settings, 'email_provider') || 'resend').trim().toLowerCase();

  if (!hasValue(settings, 'email_from_address')) {
    return { ready: false, reason: 'Configure email sender address first.' };
  }

  if (provider === 'smtp') {
    const smtpReady = [
      'email_smtp_host',
      'email_smtp_port',
      'email_smtp_username',
      'email_smtp_password',
    ].every((key) => hasValue(settings, key));

    return smtpReady
      ? { ready: true, reason: null }
      : { ready: false, reason: 'Complete SMTP configuration first.' };
  }

  if (provider === 'resend' || provider === 'sendgrid') {
    return hasValue(settings, 'email_api_key')
      ? { ready: true, reason: null }
      : { ready: false, reason: 'Add the email API key first.' };
  }

  return { ready: false, reason: 'Choose a supported email provider first.' };
}

export function getNotificationChannelReadiness(
  settings: WhatsAppGatewaySettingsMap,
): NotificationChannelReadiness {
  return {
    whatsapp: getWhatsAppReadiness(settings),
    email: getEmailReadiness(settings),
    in_app: { ready: true, reason: null },
  };
}

export function sanitizeWhatsAppEventPreference(
  preference: WhatsAppEventPreference,
  readiness: NotificationChannelReadiness,
): WhatsAppEventPreference {
  return {
    whatsapp: readiness.whatsapp.ready ? preference.whatsapp : false,
    email: readiness.email.ready ? preference.email : false,
    in_app: preference.in_app,
  };
}
