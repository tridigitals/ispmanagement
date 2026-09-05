<script lang="ts">
  /*
    Templat pesan v2 — gelombang 24b.

    Versi lama: (app)/admin/message-templates/+page.svelte (685 baris).
    Perilaku dipertahankan identik: filter q/useCase/channel/status,
    kartu daftar, Modal editor (form + panel variabel + pratinjau
    dengan konteks contoh), hapus via ConfirmDialog.
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type {
    MessageTemplate,
    MessageTemplateChannel,
    MessageTemplatePayload,
    MessageTemplateStatus,
    MessageTemplateTriggerMode,
  } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    Field,
    Icon,
    PageHeader,
  } from '$lib/components/ds';

  const variableGroups = [
    { label: 'Tenant', variables: ['tenant.name'] },
    {
      label: 'Pelanggan',
      variables: ['customer.id', 'customer.name', 'customer.email', 'customer.phone', 'customer.status', 'customer.notes'],
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

  const canRead = $derived($can('read', 'communication_templates') || $can('manage', 'communication_templates'));
  const canManage = $derived($can('manage', 'communication_templates'));

  let showDeleteConfirm = $state(false);
  let deleteTargetId = $state<string | null>(null);

  const useCaseOptions = [
    { value: 'all', label: 'Semua keperluan' },
    { value: 'billing', label: 'Tagihan' },
    { value: 'installation', label: 'Instalasi' },
    { value: 'support', label: 'Dukungan' },
    { value: 'outage', label: 'Gangguan' },
    { value: 'lifecycle', label: 'Siklus hidup' },
    { value: 'custom', label: 'Kustom' },
  ];
  const channelOptions = [
    { value: 'all', label: 'Semua kanal' },
    { value: 'whatsapp', label: 'WhatsApp' },
    { value: 'email', label: 'Email' },
    { value: 'both', label: 'Keduanya' },
  ];
  const statusOptions = [
    { value: 'all', label: 'Semua status' },
    { value: 'draft', label: 'Draf' },
    { value: 'active', label: 'Aktif' },
    { value: 'archived', label: 'Arsip' },
  ];
  const statusFormOptions = statusOptions.filter((o) => o.value !== 'all');
  const triggerOptions = [
    { value: 'manual', label: 'Manual' },
    { value: 'automatic', label: 'Otomatis' },
    { value: 'both', label: 'Keduanya' },
  ];

  function templateStatusTone(s: string): 'positive' | 'warning' | 'neutral' {
    if (s === 'active') return 'positive';
    if (s === 'draft') return 'warning';
    return 'neutral';
  }

  onMount(async () => {
    if (!canRead) {
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
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
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
    if (!canManage || saving) return;
    if (!form.key.trim() || !form.name.trim()) {
      toast.error('Kunci dan nama templat wajib diisi.');
      return;
    }
    saving = true;
    try {
      if (editing) await api.messageTemplates.update(editing.id, form);
      else await api.messageTemplates.create(form);
      toast.success('Templat pesan disimpan.');
      showEditor = false;
      await loadTemplates();
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      saving = false;
    }
  }

  function confirmDeleteTemplate(template: MessageTemplate) {
    if (!canManage) return;
    deleteTargetId = template.id;
    showDeleteConfirm = true;
  }

  async function handleConfirmDelete() {
    if (!deleteTargetId) return;
    try {
      await api.messageTemplates.delete(deleteTargetId);
      toast.success('Templat pesan dihapus.');
      await loadTemplates();
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      showDeleteConfirm = false;
      deleteTargetId = null;
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
    } catch (e) {
      preview = null;
      toast.error(extractApiErrorMessage(e));
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
<AppShell title="Templat pesan">
  <PageHeader
    title="Templat pesan"
    eyebrow="Komunikasi"
    desc="Templat WhatsApp & email dengan variabel pelanggan."
  >
    {#snippet actions()}
      {#if canManage}
        <Button variant="primary" onclick={openCreate}>Templat baru</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <Card title="Filter">
    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
      <Field id="mt-q" label="Cari" type="text" stacked value={q} onchange={(v) => { q = v; void loadTemplates(); }} placeholder="Cari kunci/nama…" />
      <Field id="mt-usecase" label="Keperluan" type="select" stacked value={useCase} options={useCaseOptions} onchange={(v) => { useCase = v; void loadTemplates(); }} />
      <Field id="mt-channel" label="Kanal" type="select" stacked value={channel} options={channelOptions} onchange={(v) => { channel = v as MessageTemplateChannel | 'all'; void loadTemplates(); }} />
      <Field id="mt-status" label="Status" type="select" stacked value={status} options={statusOptions} onchange={(v) => { status = v as MessageTemplateStatus | 'all'; void loadTemplates(); }} />
      <div class="flex items-end">
        <Button variant="ghost" icon="refresh" onclick={() => void loadTemplates()} disabled={loading}>
          Segarkan
        </Button>
      </div>
    </div>
  </Card>

  {#if loading}
    <Card><div class="py-10 text-center text-sm text-ink-500">Memuat templat…</div></Card>
  {:else if templates.length === 0}
    <Card>
      <div class="py-10 text-center">
        <div class="text-sm font-medium text-ink-900">Belum ada templat</div>
        <p class="mt-1 text-sm text-ink-500">Buat templat pertama untuk keperluan ini.</p>
      </div>
    </Card>
  {:else}
    <div class="grid gap-3 md:grid-cols-2">
      {#each templates as template (template.id)}
        <Card>
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <span class="truncate text-sm font-semibold text-ink-900">{template.name}</span>
                <Badge tone={templateStatusTone(template.status)} label={template.status} />
              </div>
              <p class="mt-1 truncate text-sm text-ink-500">{template.description || template.key}</p>
              <p class="mt-1.5 text-xs text-ink-400">
                {template.use_case} · {template.channel} · {template.trigger_mode} · v{template.version}
              </p>
            </div>
            <div class="flex shrink-0 gap-1">
              <Button variant="ghost" onclick={() => openEdit(template)}>Sunting</Button>
              {#if canManage}
                <Button variant="danger" onclick={() => confirmDeleteTemplate(template)}>Hapus</Button>
              {/if}
            </div>
          </div>
        </Card>
      {/each}
    </div>
  {/if}
</AppShell>

<Modal
  bind:show={showEditor}
  width="min(1040px, calc(100vw - 2rem))"
  title={editing ? `Sunting templat — ${editing.name}` : 'Templat baru'}
  onclose={() => (showEditor = false)}
>
  <div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_280px]">
    <div class="space-y-3">
      <div class="grid gap-3 sm:grid-cols-2">
        <Field id="mt-f-key" label="Kunci" type="text" stacked value={form.key} onchange={(v) => (form.key = v)} placeholder="invoice_due_reminder" />
        <Field id="mt-f-name" label="Nama" type="text" stacked value={form.name} onchange={(v) => (form.name = v)} placeholder="Pengingat jatuh tempo" />
      </div>
      <Field id="mt-f-desc" label="Deskripsi" type="text" stacked value={form.description || ''} onchange={(v) => (form.description = v)} />
      <div class="grid gap-3 sm:grid-cols-3">
        <Field id="mt-f-usecase" label="Keperluan" type="select" stacked value={form.useCase} options={useCaseOptions.filter((o) => o.value !== 'all')} onchange={(v) => (form.useCase = v)} />
        <Field id="mt-f-channel" label="Kanal" type="select" stacked value={form.channel} options={channelOptions.filter((o) => o.value !== 'all')} onchange={(v) => (form.channel = v as MessageTemplateChannel)} />
        <Field id="mt-f-status" label="Status" type="select" stacked value={form.status} options={statusFormOptions} onchange={(v) => (form.status = v as MessageTemplateStatus)} />
      </div>
      <div class="grid gap-3 sm:grid-cols-2">
        <Field id="mt-f-trigger" label="Pemicu" type="select" stacked value={form.triggerMode} options={triggerOptions} onchange={(v) => (form.triggerMode = v as MessageTemplateTriggerMode)} />
        <Field id="mt-f-event" label="Kunci event" type="text" stacked value={form.eventKey || ''} onchange={(v) => (form.eventKey = v)} placeholder="invoice.due_reminder" />
      </div>
      <Field id="mt-f-wa" label="Isi WhatsApp" type="textarea" stacked rows={6} value={form.whatsappBody || ''} onchange={(v) => (form.whatsappBody = v)} />
      <Field id="mt-f-subject" label="Subjek email" type="text" stacked value={form.emailSubject || ''} onchange={(v) => (form.emailSubject = v)} />
      <Field id="mt-f-email" label="Isi email" type="textarea" stacked rows={8} value={form.emailBody || ''} onchange={(v) => (form.emailBody = v)} />
    </div>
    <aside class="space-y-3 rounded-xl bg-ink-50 p-3">
      <div>
        <div class="text-sm font-semibold text-ink-900">Variabel</div>
        <p class="text-xs text-ink-500">Klik untuk menyisipkan ke isi pesan.</p>
      </div>
      {#each variableGroups as group}
        <div>
          <div class="text-xs font-medium text-ink-500">{group.label}</div>
          <div class="mt-1 flex flex-wrap gap-1.5">
            {#each group.variables as variable}
              <button
                type="button"
                class="focus-ring rounded-md bg-white px-2 py-1 font-mono text-xs text-ink-700 ring-1 ring-ink-200 hover:bg-ink-100"
                onclick={() => insertVariable(variable)}
              >
                {'{{'}{variable}{'}}'}
              </button>
            {/each}
          </div>
        </div>
      {/each}
      <Button variant="secondary" onclick={() => void previewTemplate()} disabled={previewing}>
        {previewing ? 'Mempratinjau…' : 'Pratinjau'}
      </Button>
      {#if preview}
        <div class="space-y-1.5 rounded-lg bg-white p-3 text-sm ring-1 ring-ink-200">
          <div class="text-xs font-medium text-ink-500">Pratinjau</div>
          {#if preview.whatsappBody}<p class="whitespace-pre-wrap">{preview.whatsappBody}</p>{/if}
          {#if preview.emailSubject}<p><b>{preview.emailSubject}</b></p>{/if}
          {#if preview.emailBody}<p class="whitespace-pre-wrap">{preview.emailBody}</p>{/if}
          <p class="text-xs text-ink-400">Variabel: {preview.variables.join(', ') || '-'}</p>
        </div>
      {/if}
    </aside>
  </div>
  <div class="mt-4 flex justify-end gap-2">
    <Button variant="ghost" onclick={() => (showEditor = false)}>Batal</Button>
    <Button variant="primary" onclick={() => void saveTemplate()} disabled={!canManage || saving}>
      {saving ? 'Menyimpan…' : 'Simpan'}
    </Button>
  </div>
</Modal>

<ConfirmDialog
  bind:show={showDeleteConfirm}
  title="Hapus templat?"
  message="Templat yang dihapus tidak bisa dikembalikan."
  confirmText="Hapus"
  cancelText="Batal"
  type="danger"
  onconfirm={() => void handleConfirmDelete()}
  oncancel={() => { deleteTargetId = null; }}
/>
