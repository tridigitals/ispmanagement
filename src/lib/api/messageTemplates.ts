import { getTokenOrThrow, safeInvoke } from './core';
import type {
  MessageTemplate,
  MessageTemplateListFilters,
  MessageTemplatePayload,
  MessageTemplatePreviewRequest,
  MessageTemplatePreviewResponse,
} from './types';

export const messageTemplates = {
  list: (filters: MessageTemplateListFilters = {}): Promise<MessageTemplate[]> =>
    safeInvoke('list_message_templates', {
      token: getTokenOrThrow(),
      ...filters,
    }).then((templates: unknown) => (templates as any[]).map(normalizeMessageTemplate)),

  create: (payload: MessageTemplatePayload): Promise<MessageTemplate> =>
    safeInvoke('create_message_template', {
      token: getTokenOrThrow(),
      payload,
    }).then(normalizeMessageTemplate),

  update: (id: string, payload: MessageTemplatePayload): Promise<MessageTemplate> =>
    safeInvoke('update_message_template', {
      token: getTokenOrThrow(),
      id,
      payload,
    }).then(normalizeMessageTemplate),

  delete: (id: string): Promise<boolean> =>
    safeInvoke('delete_message_template', {
      token: getTokenOrThrow(),
      id,
    }),

  preview: (payload: MessageTemplatePreviewRequest): Promise<MessageTemplatePreviewResponse> =>
    safeInvoke('preview_message_template', {
      token: getTokenOrThrow(),
      payload,
    }),
};

function normalizeMessageTemplate(template: any): MessageTemplate {
  return {
    ...template,
    tenant_id: template.tenant_id ?? template.tenantId,
    use_case: template.use_case ?? template.useCase,
    trigger_mode: template.trigger_mode ?? template.triggerMode,
    event_key: template.event_key ?? template.eventKey ?? null,
    whatsapp_body: template.whatsapp_body ?? template.whatsappBody ?? null,
    email_subject: template.email_subject ?? template.emailSubject ?? null,
    email_body: template.email_body ?? template.emailBody ?? null,
    created_at: template.created_at ?? template.createdAt,
    updated_at: template.updated_at ?? template.updatedAt,
  };
}
