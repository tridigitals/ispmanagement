<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { toast } from 'svelte-sonner';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import {
    api,
    type MessageTemplate,
    type MessageTemplateChannel,
    type MessageTemplatePayload,
    type MessageTemplateStatus,
    type MessageTemplateTriggerMode,
  } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';

  const variableGroups = [
    {
      labelKey: 'admin.message_templates.variables.tenant',
      variables: ['tenant.name'],
    },
    {
      labelKey: 'admin.message_templates.variables.customer',
      variables: [
        'customer.id',
        'customer.name',
        'customer.email',
        'customer.phone',
        'customer.status',
        'customer.notes',
      ],
    },
  ];

  const sampleContext = {
    tenant: { name: 'Tri Digitals' },
    customer: {
      id: 'cust-001',
      name: 'Andi Pratama',
      email: 'andi@example.com',
      phone: '08123456789',
      status: 'active',
      notes: 'Customer prioritas',
    },
  };

  let templates = $state<MessageTemplate[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let previewing = $state(false);
  let showEditor = $state(false);
  let editing = $state<MessageTemplate | null>(null);
  let preview = $state<{ whatsappBody?: string | null; emailSubject?: string | null; emailBody?: string | null; variables: string[] } | null>(null);

  let q = $state('');
  let channel = $state<MessageTemplateChannel | 'all'>('all');
  let status = $state<MessageTemplateStatus | 'all'>('all');
  let useCase = $state('all');

  let form = $state<MessageTemplatePayload>(emptyForm());

  const canReadTemplates = $derived(
    $can('read', 'communication_templates') || $can('manage', 'communication_templates'),
  );
  const canManageTemplates = $derived($can('manage', 'communication_templates'));

  onMount(async () => {
    if (!canReadTemplates) {
      goto('/unauthorized');
      return;
    }
    await loadTemplates();
  });

  function emptyForm(): MessageTemplatePayload {
    return {
      key: '',
      name: '',
      description: '',
      useCase: 'lifecycle',
      target: 'customer',
      triggerMode: 'manual',
      eventKey: '',
      channel: 'whatsapp',
      locale: 'id-ID',
      status: 'draft',
      whatsappBody: '',
      emailSubject: '',
      emailBody: '',
    };
  }

  async function loadTemplates() {
    loading = true;
    try {
      templates = await api.messageTemplates.list({
        q: q.trim() || undefined,
        channel,
        status,
        useCase,
        target: 'customer',
      });
    } catch (e: any) {
      toast.error(e?.message || $t('admin.message_templates.toasts.load_failed') || 'Failed to load message templates');
    } finally {
      loading = false;
    }
  }

  function openCreate() {
    editing = null;
    form = emptyForm();
    preview = null;
    showEditor = true;
  }

  function openEdit(template: MessageTemplate) {
    editing = template;
    form = {
      key: template.key,
      name: template.name,
      description: template.description || '',
      useCase: template.use_case,
      target: template.target,
      triggerMode: template.trigger_mode as MessageTemplateTriggerMode,
      eventKey: template.event_key || '',
      channel: template.channel as MessageTemplateChannel,
      locale: template.locale,
      status: template.status as MessageTemplateStatus,
      whatsappBody: template.whatsapp_body || '',
      emailSubject: template.email_subject || '',
      emailBody: template.email_body || '',
    };
    preview = null;
    showEditor = true;
  }

  async function saveTemplate() {
    if (!canManageTemplates || saving) return;
    if (!form.key.trim() || !form.name.trim()) {
      toast.error($t('admin.message_templates.toasts.required') || 'Template key and name are required');
      return;
    }
    saving = true;
    try {
      if (editing) await api.messageTemplates.update(editing.id, form);
      else await api.messageTemplates.create(form);
      toast.success($t('admin.message_templates.toasts.saved') || 'Message template saved');
      showEditor = false;
      await loadTemplates();
    } catch (e: any) {
      toast.error(e?.message || $t('admin.message_templates.toasts.save_failed') || 'Failed to save message template');
    } finally {
      saving = false;
    }
  }

  async function deleteTemplate(template: MessageTemplate) {
    if (!canManageTemplates) return;
    if (!confirm(($t('admin.message_templates.confirm_delete') || 'Delete template "{name}"?').replace('{name}', template.name))) return;
    try {
      await api.messageTemplates.delete(template.id);
      toast.success($t('admin.message_templates.toasts.deleted') || 'Message template deleted');
      await loadTemplates();
    } catch (e: any) {
      toast.error(e?.message || $t('admin.message_templates.toasts.delete_failed') || 'Failed to delete message template');
    }
  }

  async function previewTemplate() {
    previewing = true;
    try {
      preview = await api.messageTemplates.preview({
        whatsappBody: form.whatsappBody,
        emailSubject: form.emailSubject,
        emailBody: form.emailBody,
        context: sampleContext,
      });
    } catch (e: any) {
      preview = null;
      toast.error(e?.message || $t('admin.message_templates.toasts.preview_failed') || 'Template preview failed');
    } finally {
      previewing = false;
    }
  }

  function insertVariable(variable: string) {
    const token = `{{${variable}}}`;
    if (form.channel === 'email') form.emailBody = `${form.emailBody || ''}${token}`;
    else form.whatsappBody = `${form.whatsappBody || ''}${token}`;
  }
</script>

<div class="page-content">
  <header class="page-header">
    <div>
      <h1>{$t('topbar.titles.message_templates') || 'Message Templates'}</h1>
      <p class="subtitle">{$t('admin.message_templates.subtitle') || 'Build granular WhatsApp and email templates for manual and automated communication.'}</p>
    </div>
    {#if canManageTemplates}
      <button class="btn btn-primary" onclick={openCreate}>
        <Icon name="plus" size={16} />
        {$t('admin.message_templates.actions.new') || 'New Template'}
      </button>
    {/if}
  </header>

  <section class="toolbar">
    <input class="input" placeholder={$t('admin.message_templates.search') || 'Search templates...'} bind:value={q} onkeydown={(e) => e.key === 'Enter' && loadTemplates()} />
    <select class="input" bind:value={useCase} onchange={loadTemplates}>
      <option value="all">{$t('admin.message_templates.filters.all_use_cases') || 'All use cases'}</option>
      <option value="billing">{$t('admin.message_templates.use_cases.billing') || 'Billing'}</option>
      <option value="installation">{$t('admin.message_templates.use_cases.installation') || 'Installation'}</option>
      <option value="support">{$t('admin.message_templates.use_cases.support') || 'Support'}</option>
      <option value="outage">{$t('admin.message_templates.use_cases.outage') || 'Outage'}</option>
      <option value="lifecycle">{$t('admin.message_templates.use_cases.lifecycle') || 'Lifecycle'}</option>
      <option value="custom">{$t('admin.message_templates.use_cases.custom') || 'Custom'}</option>
    </select>
    <select class="input" bind:value={channel} onchange={loadTemplates}>
      <option value="all">{$t('admin.message_templates.filters.all_channels') || 'All channels'}</option>
      <option value="whatsapp">WhatsApp</option>
      <option value="email">Email</option>
      <option value="both">Both</option>
    </select>
    <select class="input" bind:value={status} onchange={loadTemplates}>
      <option value="all">{$t('admin.message_templates.filters.all_statuses') || 'All statuses'}</option>
      <option value="draft">{$t('admin.message_templates.statuses.draft') || 'Draft'}</option>
      <option value="active">{$t('admin.message_templates.statuses.active') || 'Active'}</option>
      <option value="archived">{$t('admin.message_templates.statuses.archived') || 'Archived'}</option>
    </select>
    <button class="btn btn-secondary" onclick={loadTemplates} disabled={loading}>
      <Icon name="refresh-cw" size={16} />
      {$t('admin.message_templates.actions.refresh') || 'Refresh'}
    </button>
  </section>

  <section class="template-list">
    {#if loading}
      <div class="empty">{$t('admin.message_templates.loading') || 'Loading templates...'}</div>
    {:else if templates.length === 0}
      <div class="empty">{$t('admin.message_templates.empty') || 'No message templates yet.'}</div>
    {:else}
      {#each templates as template}
        <article class="template-card">
          <div>
            <div class="template-head">
              <strong>{template.name}</strong>
              <span class="pill" class:pill-green={template.status === 'active'}>{template.status}</span>
            </div>
            <p>{template.description || template.key}</p>
            <div class="meta">
              <span>{template.use_case}</span>
              <span>{template.channel}</span>
              <span>{template.trigger_mode}</span>
              <span>v{template.version}</span>
            </div>
          </div>
          <div class="actions">
            <button class="btn-icon" title="Edit" onclick={() => openEdit(template)}>
              <Icon name="pencil" size={16} />
            </button>
            {#if canManageTemplates}
              <button class="btn-icon danger" title="Delete" onclick={() => deleteTemplate(template)}>
                <Icon name="trash-2" size={16} />
              </button>
            {/if}
          </div>
        </article>
      {/each}
    {/if}
  </section>
</div>

<Modal
  show={showEditor}
  width="min(1040px, calc(100vw - 2rem))"
  title={editing ? ($t('admin.message_templates.editor.edit_title') || 'Edit Template') : ($t('admin.message_templates.editor.new_title') || 'New Template')}
  onclose={() => (showEditor = false)}
>
  <div class="editor-shell">
    <div class="editor-intro">
      <div>
        <strong>{$t('admin.message_templates.editor.content_title') || 'Template content'}</strong>
        <p>{$t('admin.message_templates.editor.content_hint') || 'Use variables for customer-specific messages. Keep WhatsApp concise and email more complete.'}</p>
      </div>
      <span class="editor-badge">{editing ? `v${editing.version}` : $t('admin.message_templates.editor.draft_badge') || 'Draft'}</span>
    </div>

    <div class="editor-grid">
      <div class="form">
      <div class="grid2">
        <label><span>{$t('admin.message_templates.fields.key') || 'Key'}</span><input class="input" bind:value={form.key} placeholder="invoice_due_reminder" /></label>
        <label><span>{$t('admin.message_templates.fields.name') || 'Name'}</span><input class="input" bind:value={form.name} placeholder="Invoice due reminder" /></label>
      </div>
      <label><span>{$t('admin.message_templates.fields.description') || 'Description'}</span><input class="input" bind:value={form.description} /></label>
      <div class="grid3">
        <label>
          <span>{$t('admin.message_templates.fields.use_case') || 'Use Case'}</span>
          <select class="input" bind:value={form.useCase}>
            <option value="billing">{$t('admin.message_templates.use_cases.billing') || 'Billing'}</option>
            <option value="installation">{$t('admin.message_templates.use_cases.installation') || 'Installation'}</option>
            <option value="support">{$t('admin.message_templates.use_cases.support') || 'Support'}</option>
            <option value="outage">{$t('admin.message_templates.use_cases.outage') || 'Outage'}</option>
            <option value="lifecycle">{$t('admin.message_templates.use_cases.lifecycle') || 'Lifecycle'}</option>
            <option value="custom">{$t('admin.message_templates.use_cases.custom') || 'Custom'}</option>
          </select>
        </label>
        <label>
          <span>{$t('admin.message_templates.fields.channel') || 'Channel'}</span>
          <select class="input" bind:value={form.channel}>
            <option value="whatsapp">WhatsApp</option>
            <option value="email">Email</option>
            <option value="both">{$t('admin.message_templates.channels.both') || 'Both'}</option>
          </select>
        </label>
        <label>
          <span>{$t('admin.message_templates.fields.status') || 'Status'}</span>
          <select class="input" bind:value={form.status}>
            <option value="draft">{$t('admin.message_templates.statuses.draft') || 'Draft'}</option>
            <option value="active">{$t('admin.message_templates.statuses.active') || 'Active'}</option>
            <option value="archived">{$t('admin.message_templates.statuses.archived') || 'Archived'}</option>
          </select>
        </label>
      </div>
      <div class="grid2">
        <label>
          <span>{$t('admin.message_templates.fields.trigger') || 'Trigger'}</span>
          <select class="input" bind:value={form.triggerMode}>
            <option value="manual">{$t('admin.message_templates.triggers.manual') || 'Manual'}</option>
            <option value="automatic">{$t('admin.message_templates.triggers.automatic') || 'Automatic'}</option>
            <option value="both">{$t('admin.message_templates.triggers.both') || 'Both'}</option>
          </select>
        </label>
        <label><span>{$t('admin.message_templates.fields.event_key') || 'Event Key'}</span><input class="input" bind:value={form.eventKey} placeholder="invoice.due_reminder" /></label>
      </div>
      <label><span>{$t('admin.message_templates.fields.whatsapp_body') || 'WhatsApp Body'}</span><textarea class="input message-area" rows="6" bind:value={form.whatsappBody}></textarea></label>
      <label><span>{$t('admin.message_templates.fields.email_subject') || 'Email Subject'}</span><input class="input" bind:value={form.emailSubject} /></label>
      <label><span>{$t('admin.message_templates.fields.email_body') || 'Email Body'}</span><textarea class="input message-area email-area" rows="8" bind:value={form.emailBody}></textarea></label>
      </div>

      <aside class="side-panel">
        <strong>{$t('admin.message_templates.variables.title') || 'Variable Picker'}</strong>
        <p>{$t('admin.message_templates.variables.hint') || 'Click a token to insert it into the active channel body.'}</p>
      {#each variableGroups as group}
        <div class="variable-group">
          <span>{$t(group.labelKey)}</span>
          {#each group.variables as variable}
            <button class="variable-chip" onclick={() => insertVariable(variable)}>
              {'{{'}{variable}{'}}'}
            </button>
          {/each}
        </div>
      {/each}
      <button class="btn btn-secondary" onclick={previewTemplate} disabled={previewing}>
        <Icon name="eye" size={16} />
        {previewing ? ($t('admin.message_templates.actions.previewing') || 'Previewing...') : ($t('admin.message_templates.actions.preview') || 'Preview')}
      </button>
      {#if preview}
        <div class="preview">
          <strong>{$t('admin.message_templates.preview.title') || 'Preview'}</strong>
          {#if preview.whatsappBody}<p>{preview.whatsappBody}</p>{/if}
          {#if preview.emailSubject}<p><b>{preview.emailSubject}</b></p>{/if}
          {#if preview.emailBody}<p>{preview.emailBody}</p>{/if}
          <small>{$t('admin.message_templates.preview.variables') || 'Variables'}: {preview.variables.join(', ') || '-'}</small>
        </div>
      {/if}
      </aside>
    </div>
  </div>
  <div class="modal-actions">
    <button class="btn btn-secondary" onclick={() => (showEditor = false)}>{$t('admin.message_templates.actions.cancel') || 'Cancel'}</button>
    <button class="btn btn-primary" onclick={saveTemplate} disabled={!canManageTemplates || saving}>
      <Icon name="save" size={16} />
      {saving ? ($t('admin.message_templates.actions.saving') || 'Saving...') : ($t('admin.message_templates.actions.save') || 'Save')}
    </button>
  </div>
</Modal>

<style>
  .page-content {
    padding: 1.25rem 1.5rem 1.5rem;
  }

  .page-header,
  .toolbar,
  .template-card,
  .template-head,
  .actions,
  .modal-actions {
    display: flex;
    gap: 0.75rem;
  }

  .page-header {
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  .subtitle,
  .template-card p,
  .meta,
  label > span,
  .variable-group span,
  .preview small {
    color: var(--text-secondary);
  }

  .toolbar {
    align-items: center;
    flex-wrap: wrap;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    padding: 0.85rem;
    margin-bottom: 1rem;
  }

  .input {
    width: 100%;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 0.6rem 0.7rem;
    outline: none;
  }

  .toolbar .input {
    max-width: 220px;
  }

  .btn,
  .btn-icon,
  .variable-chip {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 10px;
    cursor: pointer;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
    padding: 0.58rem 0.85rem;
  }

  .btn-primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .btn-icon {
    padding: 0.45rem;
  }

  .btn-icon.danger {
    color: rgb(239, 68, 68);
    border-color: rgba(239, 68, 68, 0.35);
  }

  .template-list,
  .editor-shell,
  .form,
  .side-panel,
  .variable-group {
    display: grid;
    gap: 0.75rem;
  }

  .template-card {
    justify-content: space-between;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    padding: 0.9rem;
  }

  .template-head {
    align-items: center;
    margin-bottom: 0.25rem;
  }

  .pill {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 0.2rem 0.45rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .pill-green {
    color: rgb(34, 197, 94);
    border-color: rgba(34, 197, 94, 0.35);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    font-size: 0.82rem;
  }

  .empty {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    color: var(--text-secondary);
    padding: 1rem;
    text-align: center;
  }

  .editor-shell {
    gap: 1rem;
  }

  .editor-intro {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-surface) 88%, var(--bg-app));
    padding: 0.85rem 1rem;
  }

  .editor-intro p {
    margin: 0.25rem 0 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
    line-height: 1.45;
  }

  .editor-badge {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    color: var(--text-secondary);
    font-size: 0.78rem;
    padding: 0.25rem 0.55rem;
    white-space: nowrap;
  }

  .editor-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 300px;
    gap: 1rem;
    align-items: start;
  }

  .grid2,
  .grid3 {
    display: grid;
    gap: 0.75rem;
  }

  .grid2 {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .grid3 {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  label > span {
    display: block;
    margin-bottom: 0.35rem;
    font-size: 0.86rem;
  }

  textarea.input {
    resize: vertical;
    line-height: 1.5;
  }

  .message-area {
    min-height: 132px;
  }

  .email-area {
    min-height: 180px;
  }

  .side-panel {
    align-content: start;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    padding: 0.85rem;
    position: sticky;
    top: 0;
  }

  .side-panel p {
    margin: -0.25rem 0 0;
    color: var(--text-secondary);
    font-size: 0.84rem;
    line-height: 1.4;
  }

  .variable-chip {
    padding: 0.45rem 0.55rem;
    text-align: left;
    font-size: 0.82rem;
  }

  .preview {
    border-top: 1px solid var(--border-color);
    padding-top: 0.75rem;
  }

  .preview p {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .modal-actions {
    justify-content: flex-end;
    margin-top: 1rem;
  }

  @media (max-width: 720px) {
    .page-content {
      padding: 1rem;
    }

    .page-header,
    .template-card,
    .modal-actions {
      flex-direction: column;
      align-items: stretch;
    }

    .toolbar .input {
      max-width: none;
    }

    .editor-grid,
    .grid2,
    .grid3 {
      grid-template-columns: 1fr;
    }

    .editor-intro {
      flex-direction: column;
    }

    .side-panel {
      position: static;
    }
  }
</style>
