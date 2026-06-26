<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import type {
    CustomerSubscriptionView,
    Invoice,
    InstallationWorkOrderView,
    WorkOrderRescheduleRequestView,
  } from '$lib/api/client';

  let {
    trackerLoading,
    trackerError,
    trackerSub,
    trackerWo,
    trackerReschedule,
    trackerInvoice,
    rescheduleAt = $bindable(),
    rescheduleReason = $bindable(),
    rescheduleBusy,
    tt,
    formatDate,
    formatCurrency,
    invoiceActionLabel,
    invoiceStatusTone,
    stepState,
    showRescheduleInfo,
    rescheduleStatusMeta,
    canRequestReschedule,
    onClose,
    onOpenTrackerInvoice,
    onSubmitReschedule,
  }: {
    trackerLoading: boolean;
    trackerError: string;
    trackerSub: CustomerSubscriptionView | null;
    trackerWo: InstallationWorkOrderView | null;
    trackerReschedule: WorkOrderRescheduleRequestView | null;
    trackerInvoice: Invoice | null;
    rescheduleAt: string;
    rescheduleReason: string;
    rescheduleBusy: boolean;
    tt: (key: string, fallback: string) => string;
    formatDate: (value?: string | null) => string;
    formatCurrency: (amount: number, currencyCode?: string | null) => string;
    invoiceActionLabel: (status?: string | null, startsAt?: string | null) => string;
    invoiceStatusTone: (status?: string | null) => string;
    stepState: (step: 'requested' | 'assigned' | 'scheduled' | 'onsite' | 'active') => string;
    showRescheduleInfo: (status?: string | null) => boolean;
    rescheduleStatusMeta: (
      status?: string | null,
    ) => { tone: string; label: string } | null;
    canRequestReschedule: () => boolean;
    onClose: () => void;
    onOpenTrackerInvoice: () => void;
    onSubmitReschedule: () => void;
  } = $props();
</script>

<div
  class="tracker-backdrop"
  role="button"
  tabindex="0"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') onClose();
  }}
>
  <section class="tracker-modal">
    <header class="tracker-head">
      <div>
        <h3>{tt('dashboard.services_portal.tracker.title', 'Installation Tracker')}</h3>
        <p>{trackerSub?.package_name || trackerSub?.package_id || '-'}</p>
      </div>
      <button class="btn ghost" type="button" onclick={onClose}>
        <Icon name="x" size={14} />
        {tt('common.close', 'Close')}
      </button>
    </header>

    {#if trackerLoading}
      <div class="state-block">
        <div class="spinner"></div>
        <p>{tt('dashboard.services_portal.tracker.loading', 'Loading tracker...')}</p>
      </div>
    {:else if trackerError}
      <div class="alert">{trackerError}</div>
    {:else}
      <div class="tracker-steps">
        <div class={`step-pill ${stepState('requested')}`}>1. {tt('dashboard.services_portal.tracker.steps.requested', 'Requested')}</div>
        <div class={`step-pill ${stepState('assigned')}`}>2. {tt('dashboard.services_portal.tracker.steps.assigned', 'Assigned')}</div>
        <div class={`step-pill ${stepState('scheduled')}`}>3. {tt('dashboard.services_portal.tracker.steps.scheduled', 'Scheduled')}</div>
        <div class={`step-pill ${stepState('onsite')}`}>4. {tt('dashboard.services_portal.tracker.steps.onsite', 'On-site')}</div>
        <div class={`step-pill ${stepState('active')}`}>5. {tt('dashboard.services_portal.tracker.steps.active', 'Active')}</div>
      </div>

      <div class="tracker-grid">
        <div>
          <small>{tt('dashboard.services_portal.table.status', 'Status')}</small>
          <strong>{trackerWo?.status || '-'}</strong>
        </div>
        <div>
          <small>{tt('common.assignee', 'Assignee')}</small>
          <strong>{trackerWo?.assigned_to_name || trackerWo?.assigned_to_email || '-'}</strong>
        </div>
        <div>
          <small>{tt('dashboard.services_portal.tracker.scheduled_at', 'Scheduled At')}</small>
          <strong>{formatDate(trackerWo?.scheduled_at)}</strong>
        </div>
        <div>
          <small>{tt('dashboard.services_portal.tracker.last_update', 'Last Update')}</small>
          <strong>{formatDate(trackerWo?.updated_at || trackerSub?.updated_at)}</strong>
        </div>
      </div>

      {#if trackerInvoice}
        <section class="reschedule-status">
          <div class="reschedule-status-head">
            <h4>{tt('dashboard.services.tracker.invoice', 'Invoice')}</h4>
            <span class={`request-status ${invoiceStatusTone(trackerInvoice.status)}`}>
              {String(trackerInvoice.status || 'pending').toUpperCase()}
            </span>
          </div>
          <div class="reschedule-status-grid">
            <div>
              <small>{tt('dashboard.services.tracker.invoice_number', 'Invoice Number')}</small>
              <strong>{trackerInvoice.invoice_number || '-'}</strong>
            </div>
            <div>
              <small>{tt('dashboard.services.tracker.amount', 'Amount')}</small>
              <strong>{formatCurrency(Number(trackerInvoice.amount || 0), trackerInvoice.currency_code)}</strong>
            </div>
            <div>
              <small>{tt('dashboard.services.tracker.due_date', 'Due Date')}</small>
              <strong>{formatDate(trackerInvoice.due_date)}</strong>
            </div>
            <div>
              <small>{tt('dashboard.services.tracker.paid_at', 'Paid At')}</small>
              <strong>{formatDate(trackerInvoice.paid_at)}</strong>
            </div>
          </div>
          <div class="reschedule-actions">
            <button class="btn ghost" type="button" onclick={onOpenTrackerInvoice}>
              <Icon name="file-text" size={14} />
              {invoiceActionLabel(trackerSub?.status, trackerSub?.starts_at)}
            </button>
          </div>
        </section>
      {/if}

      {#if trackerReschedule && showRescheduleInfo(trackerSub?.status)}
        <section class="reschedule-status">
          <div class="reschedule-status-head">
            <h4>
              {tt(
                'dashboard.services_portal.reschedule.latest_title',
                'Latest Reschedule Request',
              )}
            </h4>
            <span class={`request-status ${trackerReschedule.status || 'pending'}`}>
              {String(trackerReschedule.status || 'pending').toUpperCase()}
            </span>
          </div>
          <div class="reschedule-status-grid">
            <div>
              <small>
                {tt(
                  'dashboard.services_portal.reschedule.labels.requested_schedule',
                  'Requested Schedule',
                )}
              </small>
              <strong>{formatDate(trackerReschedule.requested_schedule_at)}</strong>
            </div>
            <div>
              <small>
                {tt('dashboard.services_portal.reschedule.labels.requested_by', 'Requested By')}
              </small>
              <strong>{trackerReschedule.requested_by_name || trackerReschedule.requested_by_email || '-'}</strong>
            </div>
          </div>
          {#if trackerReschedule.reason}
            <p>
              <strong>{tt('dashboard.services_portal.reschedule.labels.reason', 'Reason')}:</strong>
              {trackerReschedule.reason}
            </p>
          {/if}
          {#if trackerReschedule.review_notes}
            <p>
              <strong>{tt('dashboard.services_portal.reschedule.labels.admin_notes', 'Admin Notes')}:</strong>
              {trackerReschedule.review_notes}
            </p>
          {/if}
        </section>
      {/if}

      {#if canRequestReschedule()}
        <section class="reschedule-box">
          <h4>{tt('dashboard.services_portal.reschedule.form.title', 'Request Reschedule')}</h4>
          <p>
            {tt(
              'dashboard.services_portal.reschedule.form.subtitle',
              'You can request new installation time before onsite work starts.',
            )}
          </p>
          <div class="reschedule-form">
            <label>
              {tt('dashboard.services_portal.reschedule.form.new_schedule', 'New Schedule')}
              <input type="datetime-local" bind:value={rescheduleAt} />
            </label>
            <label>
              {tt('dashboard.services_portal.reschedule.form.reason_optional', 'Reason (optional)')}
              <textarea
                rows="3"
                bind:value={rescheduleReason}
                placeholder={tt(
                  'dashboard.services_portal.reschedule.form.reason_placeholder',
                  'Need different time slot',
                )}
              ></textarea>
            </label>
          </div>
          <div class="reschedule-actions">
            <button class="btn primary" type="button" onclick={onSubmitReschedule} disabled={rescheduleBusy}>
              <Icon name="calendar" size={14} />
              {rescheduleBusy
                ? tt('dashboard.services_portal.reschedule.form.submitting', 'Submitting...')
                : tt('dashboard.services_portal.reschedule.form.submit', 'Submit Reschedule')}
            </button>
          </div>
        </section>
      {:else if trackerWo?.status === 'pending' && !trackerWo?.scheduled_at}
        <section class="reschedule-box">
          <h4>{tt('dashboard.services_portal.reschedule.form.title', 'Request Reschedule')}</h4>
          <p>
            {tt(
              'dashboard.services_portal.reschedule.form.wait_schedule',
              'Reschedule will be available after admin/technician sets your installation schedule.',
            )}
          </p>
        </section>
      {/if}
    {/if}
  </section>
</div>

<style>
  .btn {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.62rem 0.9rem;
    font-weight: 800;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
  }

  .btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .btn.primary {
    background: var(--color-primary);
    border-color: color-mix(in srgb, var(--color-primary) 55%, var(--border-color));
    color: white;
  }

  .btn.ghost {
    background: color-mix(in srgb, var(--bg-surface) 75%, transparent);
  }

  .btn:disabled {
    opacity: 0.66;
    cursor: not-allowed;
  }

  .alert {
    border: 1px solid rgba(239, 68, 68, 0.35);
    background: rgba(239, 68, 68, 0.08);
    color: #fecaca;
    padding: 0.85rem 1rem;
    border-radius: 12px;
    font-weight: 600;
  }

  .state-block {
    border: 1px dashed var(--border-color);
    border-radius: 12px;
    min-height: 120px;
    display: grid;
    place-content: center;
    text-align: center;
    gap: 0.7rem;
    color: var(--text-secondary);
    padding: 1.2rem;
  }

  .spinner {
    width: 28px;
    height: 28px;
    border-radius: 999px;
    border: 2px solid color-mix(in srgb, var(--text-secondary) 35%, transparent);
    border-top-color: var(--color-primary);
    animation: spin 0.8s linear infinite;
    margin: 0 auto;
  }

  .tracker-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(2, 6, 18, 0.72);
    display: grid;
    place-items: center;
    z-index: 1200;
    padding: 18px;
  }

  .tracker-modal {
    width: min(860px, 100%);
    max-height: calc(100vh - 36px);
    overflow: auto;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: #0d1323;
    padding: 14px;
    display: grid;
    gap: 12px;
  }

  .tracker-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }

  .tracker-head h3 {
    margin: 0;
    color: var(--text-primary);
  }

  .tracker-head p {
    margin: 4px 0 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
  }

  .tracker-steps {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 8px;
  }

  .step-pill {
    border-radius: 999px;
    border: 1px solid #334155;
    color: #9ca9c2;
    font-size: 0.78rem;
    padding: 7px 10px;
    text-align: center;
    font-weight: 700;
  }

  .step-pill.done {
    border-color: rgba(34, 197, 94, 0.4);
    background: rgba(34, 197, 94, 0.14);
    color: #86efac;
  }

  .tracker-grid {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: #11192d;
    padding: 10px;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .tracker-grid small,
  .reschedule-status-grid small {
    display: block;
    color: var(--text-secondary);
    margin-bottom: 4px;
    font-size: 0.75rem;
  }

  .tracker-grid strong,
  .reschedule-status-grid strong {
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .reschedule-status {
    border: 1px solid rgba(234, 179, 8, 0.36);
    background: rgba(120, 53, 15, 0.12);
    border-radius: 12px;
    padding: 10px;
    display: grid;
    gap: 8px;
  }

  .reschedule-status-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .reschedule-status-head h4,
  .reschedule-box h4 {
    margin: 0;
    color: var(--text-primary);
  }

  .request-status {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 0.2rem 0.55rem;
    font-size: 0.72rem;
    font-weight: 800;
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    background: rgba(148, 163, 184, 0.12);
  }

  .request-status.pending {
    border-color: rgba(245, 158, 11, 0.5);
    color: #facc15;
    background: rgba(161, 98, 7, 0.18);
  }

  .request-status.approved {
    border-color: rgba(34, 197, 94, 0.5);
    color: #86efac;
    background: rgba(21, 128, 61, 0.18);
  }

  .request-status.rejected {
    border-color: rgba(239, 68, 68, 0.5);
    color: #fca5a5;
    background: rgba(185, 28, 28, 0.2);
  }

  .reschedule-status-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .reschedule-status p,
  .reschedule-box p,
  .reschedule-form label {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.84rem;
  }

  .reschedule-box {
    border: 1px solid rgba(99, 102, 241, 0.32);
    background: rgba(99, 102, 241, 0.09);
    border-radius: 12px;
    padding: 10px;
    display: grid;
    gap: 8px;
  }

  .reschedule-form {
    display: grid;
    gap: 8px;
  }

  .reschedule-form label {
    display: grid;
    gap: 6px;
  }

  .reschedule-form input,
  .reschedule-form textarea {
    border: 1px solid var(--border-color);
    background: #0b1220;
    color: var(--text-primary);
    border-radius: 10px;
    padding: 8px 10px;
  }

  .reschedule-actions {
    display: flex;
    justify-content: flex-end;
  }

  @media (max-width: 900px) {
    .tracker-modal {
      padding: 12px;
    }

    .tracker-head {
      flex-direction: column;
      align-items: flex-start;
    }

    .tracker-steps,
    .tracker-grid,
    .reschedule-status-grid {
      grid-template-columns: 1fr;
    }

    .reschedule-actions {
      justify-content: stretch;
    }

    .reschedule-actions .btn {
      width: 100%;
      justify-content: center;
    }
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
