<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import InstallationCableMap from '$lib/components/network/InstallationCableMap.svelte';

  let {
    tr,
    canReadAuditLogs,
    canManageWorkOrders,
    isAdminOwner,
    detailOpen = $bindable(),
    activeRow,
    closeDetail,
    statusClass,
    effectiveStep,
    checkCable = $bindable(),
    checkOnt = $bindable(),
    checkPppoe = $bindable(),
    checkSpeed = $bindable(),
    onsiteActiveIndex,
    onsiteActiveTask,
    checklistDoneCount,
    checklistTotal,
    isGraceActive,
    subscriptionGraceDeadlineLabel,
    currentFocusTitle,
    currentFocusHint,
    subscriptionStatusLabel,
    formAssignee = $bindable(),
    assigneeOptions,
    busyId,
    formNotes = $bindable(),
    canReleaseRow,
    canSaveAssignStep,
    saveAssignStep,
    activeDeferredTabLoading,
    rescheduleLoading,
    rescheduleRequest,
    formatDateTime,
    canReviewReschedule,
    rescheduleOverrideAt = $bindable(),
    rescheduleDecisionNotes = $bindable(),
    rescheduleDecisionBusy,
    approveRescheduleFromDetail,
    rejectRescheduleFromDetail,
    assigneeLabel,
    resetToAssignStep,
    formSchedule = $bindable(),
    canSaveScheduleStep,
    saveScheduleStep,
    canStartActive,
    startFromDetail,
    showCableMapDrawer = $bindable(),
    openCableDesigner,
    handleCableMapSaved,
    installationPhotos,
    uploadingPhotos,
    uploadInstallationPhotos,
    removeInstallationPhoto,
    getStorageContentUrl,
    loadingInstallationPppoe,
    installationSubscription,
    installationPppoeUsername = $bindable(),
    installationPppoePassword = $bindable(),
    installationPppoeComment = $bindable(),
    installationPppoeTarget = $bindable(),
    installationPppoeTargetOptions,
    installationManagedRadiusHint,
    installationManagedRadiusLoadError,
    installationManagedRadiusSetup,
    installationPppoeAccount,
    savingInstallationPppoe,
    saveInstallationPppoe,
    applyInstallationPppoe,
    installationPppoeMapping,
    getOnsiteTaskChecked,
    setOnsiteTaskChecked,
    goPrevOnsiteStep,
    goNextOnsiteStep,
    markActiveOnsiteStepDone,
    savePlan,
    canCompleteActive,
    completeFromDetail,
    isClosedState,
    isAwaitingFirstPayment,
    canCreateMissingInvoice,
    creatingInvoiceId,
    createInvoiceFromDetail,
    setStatus,
    timelineLoading,
    timelineRows,
    canOperateRow,
    isPlanReady,
    claimWorkOrder,
    isUnassigned,
    isAssignedToCurrentUser,
    releaseWorkOrder,
    openCancelDialog,
    cancelDialogOpen = $bindable(),
    cancelTarget,
    cancelReason = $bindable(),
    closeCancelDialog,
    confirmCancelFromDialog,
    hasValidCancelReason,
  } = $props();
</script>

{#if detailOpen && activeRow}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    onclick={(e) => {
      if (e.target === e.currentTarget) closeDetail();
    }}
    onkeydown={(e) => {
      if (e.key === 'Escape') closeDetail();
    }}
  >
    <div class="modal">
      <div class="modal-head">
        <h2>{tr('admin.network.installations.details_title', 'Installation Details')}</h2>
        <button class="btn ghost" onclick={closeDetail}>✕</button>
      </div>

      <div class="step-flow">
        {#if activeRow.status === 'in_progress'}
          <div class:active-step={true}>1. {tr('admin.network.installations.step_assign', 'Assign')}</div>
          <div class:active-step={true}>2. {tr('admin.network.installations.step_schedule', 'Schedule')}</div>
          <div class:done-step={checkCable} class:active-step={!checkCable && onsiteActiveIndex === 0}>3. Cable</div>
          <div class:done-step={checkOnt} class:active-step={!checkOnt && onsiteActiveIndex === 1}>4. ONT</div>
          <div class:done-step={checkPppoe} class:active-step={!checkPppoe && onsiteActiveIndex === 2}>5. PPPoE</div>
          <div class:done-step={checkSpeed} class:active-step={!checkSpeed && onsiteActiveIndex === 3}>6. Speed Test</div>
          <div class:active-step={checklistDoneCount === checklistTotal}>7. {tr('admin.network.installations.step_activate', 'Activate')}</div>
        {:else}
          <div class:active-step={effectiveStep >= 1}>1. {tr('admin.network.installations.step_assign', 'Assign')}</div>
          <div class:active-step={effectiveStep >= 2}>2. {tr('admin.network.installations.step_schedule', 'Schedule')}</div>
          <div class:active-step={effectiveStep >= 3}>3. {tr('admin.network.installations.step_onsite', 'On-site & Test')}</div>
          <div class:active-step={effectiveStep >= 4}>4. {tr('admin.network.installations.step_activate', 'Finish')}</div>
        {/if}
      </div>

      <div class="meta-grid">
        <article class="meta-item"><span class="meta-label">{tr('common.customer', 'Customer')}</span><strong class="meta-value">{activeRow.customer_name || activeRow.customer_id}</strong></article>
        <article class="meta-item"><span class="meta-label">{tr('common.location', 'Location')}</span><strong class="meta-value">{activeRow.location_label || activeRow.location_id}</strong></article>
        <article class="meta-item"><span class="meta-label">{tr('common.package', 'Package')}</span><strong class="meta-value">{activeRow.package_name || '-'}</strong></article>
        <article class="meta-item"><span class="meta-label">{tr('common.status', 'Status')}</span><span class="meta-value"><span class={statusClass(activeRow.status)}>{activeRow.status}</span></span></article>
        <article class="meta-item"><span class="meta-label">{tr('admin.network.installations.subscription_status', 'Service Status')}</span><strong class="meta-value">{subscriptionStatusLabel}</strong></article>
        <article class="meta-item"><span class="meta-label">{tr('common.assignee', 'Assignee')}</span><strong class="meta-value">{activeRow.assigned_to_name || '-'}</strong></article>
      </div>

      <section class:grace={isGraceActive} class="focus-panel">
        <div class="focus-copy">
          <span class="focus-kicker">{tr('admin.network.installations.focus_kicker', 'Current Focus')}</span>
          <strong>{currentFocusTitle}</strong>
          <p>{currentFocusHint}</p>
        </div>
        {#if activeRow.status === 'completed' && isGraceActive}
          <div class="focus-chip">
            <span>{tr('admin.network.installations.grace_deadline', 'Grace active until')}</span>
            <strong>{subscriptionGraceDeadlineLabel}</strong>
          </div>
        {:else if activeRow.status === 'in_progress'}
          <div class="focus-chip">
            <span>{tr('admin.network.installations.checklist', 'Installation Checklist')}</span>
            <strong>{checklistDoneCount}/{checklistTotal}</strong>
          </div>
        {/if}
      </section>

      {#if canManageWorkOrders}
        <section class="wizard-card">
          {#if activeRow.status === 'pending' && effectiveStep === 1}
            <h3>{tr('admin.network.installations.step_assign', 'Assign')}</h3>
            {#if isAdminOwner}
              <p class="step-help">{tr('admin.network.installations.step_assign_help', 'Choose technician first, then continue to scheduling.')}</p>
              <label>
                {tr('common.assignee', 'Assignee')}
                <Select2
                  bind:value={formAssignee}
                  options={assigneeOptions}
                  placeholder={tr('admin.network.installations.assignee_placeholder', 'Select assignee')}
                  searchPlaceholder={tr('common.search', 'Search')}
                  noResultsText={tr('common.no_results', 'No results')}
                  width="100%"
                  disabled={busyId === activeRow.id || !canManageWorkOrders}
                />
              </label>
              {#if canManageWorkOrders && assigneeOptions.length === 0}
                <p class="helper-text">
                  {tr('admin.network.installations.no_assignable_members', 'No eligible installers found. Only Admin/Technician or roles with installation permission are shown.')}
                </p>
              {/if}
              <label class="notes">
                {tr('common.notes', 'Notes')}
                <textarea rows="4" bind:value={formNotes} placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}></textarea>
              </label>
              <div class="modal-actions">
                {#if canReleaseRow(activeRow)}
                  <button class="btn ghost" onclick={() => activeRow && releaseWorkOrder(activeRow)} disabled={busyId === activeRow.id}>
                    {tr('common.release', 'Release')}
                  </button>
                {/if}
                <button class="btn ghost" onclick={saveAssignStep} disabled={busyId === activeRow.id || !canSaveAssignStep}>
                  {tr('admin.network.installations.save_assign', 'Save Assignee')}
                </button>
              </div>
            {:else}
              <p class="step-help">{tr('admin.network.installations.step_take_help', 'Take this work order first, then continue to scheduling.')}</p>
              {#if isUnassigned(activeRow)}
                <div class="modal-actions">
                  <button class="btn ghost" onclick={() => activeRow && claimWorkOrder(activeRow)} disabled={busyId === activeRow.id}>
                    {tr('common.take', 'Take')}
                  </button>
                </div>
              {:else if isAssignedToCurrentUser(activeRow)}
                <p class="helper-text">{tr('admin.network.installations.already_taken_by_you', 'You already took this work order. Continue to Schedule step.')}</p>
              {:else}
                <p class="helper-text">{tr('admin.network.installations.taken_by_other', 'This work order has been taken by another technician.')}</p>
              {/if}
            {/if}
          {:else if activeRow.status === 'pending' && effectiveStep === 2}
            <h3>{tr('admin.network.installations.step_schedule', 'Schedule')}</h3>
            <p class="step-help">{tr('admin.network.installations.step_schedule_help', 'Set installation date/time, then start work order.')}</p>
            {#if rescheduleLoading}
              <p class="helper-text">{tr('common.loading', 'Loading...')}</p>
            {:else if rescheduleRequest}
              <div class="reschedule-request-card">
                <div class="reschedule-request-head">
                  <strong>{tr('admin.network.installations.reschedule_pending_title', 'Pending reschedule request')}</strong>
                  <span>{formatDateTime(rescheduleRequest.created_at)}</span>
                </div>
                <div class="reschedule-request-grid">
                  <div><span>{tr('common.requested_by', 'Requested by')}</span><strong>{rescheduleRequest.requested_by_name || rescheduleRequest.requested_by_email || '-'}</strong></div>
                  <div><span>{tr('common.schedule', 'Schedule')}</span><strong>{formatDateTime(rescheduleRequest.requested_schedule_at)}</strong></div>
                </div>
                {#if rescheduleRequest.reason}<p>{rescheduleRequest.reason}</p>{/if}
                {#if canReviewReschedule}
                  <div class="reschedule-decision-fields">
                    <label>
                      {tr('admin.network.installations.override_schedule_optional', 'Override schedule (optional)')}
                      <input type="datetime-local" bind:value={rescheduleOverrideAt} disabled={rescheduleDecisionBusy} />
                    </label>
                    <label>
                      {tr('common.notes', 'Notes')}
                      <textarea rows="3" bind:value={rescheduleDecisionNotes} placeholder={tr('admin.network.installations.reschedule_decision_notes', 'Decision notes')} disabled={rescheduleDecisionBusy}></textarea>
                    </label>
                  </div>
                  <div class="modal-actions">
                    <button class="btn ghost" type="button" onclick={approveRescheduleFromDetail} disabled={rescheduleDecisionBusy}>{tr('common.approve', 'Approve')}</button>
                    <button class="btn danger" type="button" onclick={rejectRescheduleFromDetail} disabled={rescheduleDecisionBusy}>{tr('common.reject', 'Reject')}</button>
                  </div>
                {/if}
              </div>
            {/if}
            <div class="assigned-summary">
              <span class="summary-label">{tr('common.assignee', 'Assignee')}</span>
              <strong>{assigneeLabel(formAssignee)}</strong>
              {#if isAdminOwner}<button class="btn ghost mini" type="button" onclick={resetToAssignStep}>{tr('common.edit', 'Edit')}</button>{/if}
            </div>
            <label>
              {tr('common.schedule', 'Schedule')}
              <input type="datetime-local" bind:value={formSchedule} disabled={busyId === activeRow.id} />
            </label>
            <label class="notes">
              {tr('common.notes', 'Notes')}
              <textarea rows="4" bind:value={formNotes} placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}></textarea>
            </label>
            <div class="modal-actions">
              <button class="btn ghost" onclick={saveScheduleStep} disabled={busyId === activeRow.id || !canSaveScheduleStep}>{tr('admin.network.installations.save_schedule', 'Save Schedule')}</button>
              <button class="btn" onclick={startFromDetail} disabled={busyId === activeRow.id || !canStartActive}>{tr('common.start', 'Start')}</button>
            </div>
          {:else if activeRow.status === 'in_progress' && effectiveStep === 3}
            <h3>{tr('admin.network.installations.step_onsite', 'On-site')}</h3>
            <p class="step-help">{tr('admin.network.installations.step_onsite_help', 'Complete physical installation, test internet access, then finish the visit.')}</p>
            {#if onsiteActiveTask.key === 'cable'}
              <div class="cable-designer-card">
                <div class="cable-designer-copy">
                  <strong>{tr('admin.network.installations.cable_route_title', 'Cable Route')}</strong>
                  <p>{tr('admin.network.installations.cable_route_desc', 'Draw physical cable/link route in Topology Map and save it there.')}</p>
                </div>
                <button class="btn ghost" type="button" onclick={openCableDesigner}>
                  <Icon name="map-pin" size={14} />
                  {tr('admin.network.installations.open_cable_designer', 'Draw Cable Route')}
                </button>
              </div>
              {#if showCableMapDrawer}
                <div class="cable-map-drawer">
                  <div class="cable-map-head">
                    <strong>{tr('admin.network.installations.cable_map_inline_title', 'Cable Route Designer')}</strong>
                    <button class="btn ghost mini" type="button" onclick={() => (showCableMapDrawer = false)}>{tr('common.close', 'Close')}</button>
                  </div>
                  <InstallationCableMap
                    workOrderId={activeRow.id}
                    customerId={activeRow.customer_id}
                    locationId={activeRow.location_id}
                    preferredTargetNodeId={activeRow.selected_node_id}
                    on:saved={handleCableMapSaved}
                  />
                </div>
              {/if}
            {/if}
            {#if onsiteActiveTask.key === 'pppoe'}
              <section class="pppoe-install-card">
                <div class="pppoe-install-head">
                  <div>
                    <strong>{tr('admin.network.installations.internet_test_title', 'Internet Test')}</strong>
                    <p>{tr('admin.network.installations.internet_test_help', 'Technician only enters username and password. Router, profile, and pool follow the active internet package mapping.')}</p>
                  </div>
                  {#if installationPppoeAccount}
                    <span class="status progress">{tr('admin.network.installations.internet_test_configured', 'Configured')}</span>
                  {/if}
                </div>

                {#if loadingInstallationPppoe}
                  <p class="helper-text">{tr('common.loading', 'Loading...')}</p>
                {:else if !installationSubscription && !activeRow?.package_id && !installationPppoeAccount?.package_id}
                  <p class="helper-text">Subscription internet untuk work order ini belum ditemukan.</p>
                {:else}
                  <div class="form-grid two-col compact">
                    <label class="summary-field">
                      {tr('admin.network.installations.pppoe_username', 'Username')}
                      <input class="input" bind:value={installationPppoeUsername} placeholder="pppoe username" />
                    </label>
                    <label class="summary-field">
                      {tr('admin.network.installations.pppoe_password', 'Password')}
                      <input class="input" type="password" bind:value={installationPppoePassword} placeholder={installationPppoeAccount ? tr('admin.network.installations.password_keep_existing_placeholder', 'Leave blank to keep current password') : 'pppoe password'} />
                    </label>
                  </div>

                  {#if installationPppoeTargetOptions.length > 1}
                    <div class="field">
                      <div class="field-label">{tr('admin.network.installations.provision_to', 'Provision to')}</div>
                      <select class="input" bind:value={installationPppoeTarget}>
                        {#each installationPppoeTargetOptions as option (option.value)}
                          <option value={option.value} disabled={option.disabled}>{option.label}</option>
                        {/each}
                      </select>
                      {#if installationManagedRadiusHint}
                        <p class="helper-text">
                          {tr(
                            installationManagedRadiusLoadError
                              ? 'admin.network.installations.managed_radius_load_failed'
                              : installationManagedRadiusSetup?.plan_upgrade_required
                                ? 'admin.network.installations.managed_radius_plan_required'
                                : installationManagedRadiusSetup?.tenant_has_active_assignment === false &&
                                    installationManagedRadiusSetup?.default_server_available
                                  ? 'admin.network.installations.managed_radius_assignment_inactive'
                                  : installationManagedRadiusSetup?.tenant_has_active_assignment &&
                                      installationManagedRadiusSetup?.can_create_mapping
                                    ? 'admin.network.installations.managed_radius_mapping_inactive'
                                    : 'admin.network.installations.managed_radius_not_configured',
                            installationManagedRadiusHint,
                          )}
                        </p>
                      {/if}
                    </div>
                  {/if}

                  <label class="notes">
                    {tr('admin.network.installations.pppoe_comment', 'Comment')}
                    <input class="input" bind:value={installationPppoeComment} placeholder="Optional PPPoE comment" />
                  </label>

                  {#if installationManagedRadiusSetup?.configured}
                    <p class="helper-text">{tr('admin.network.installations.managed_radius_ready_hint', 'Managed RADIUS is ready on this router. Technician can choose local router or RADIUS before applying.')}</p>
                  {/if}

                  {#if installationPppoeAccount}
                    <div class="pppoe-existing">
                      <span>{tr('admin.network.installations.pppoe_existing', 'Existing PPPoE:')}</span>
                      <strong>{installationPppoeAccount.username}</strong>
                      <span>{installationPppoeAccount.account_source === 'managed_radius' ? 'RADIUS' : 'Router'}</span>
                    </div>
                  {/if}

                  <div class="test-outcome">
                    <span class:ok={!!installationPppoeAccount} class="test-state">
                      {installationPppoeAccount
                        ? installationPppoeAccount.account_source === 'managed_radius'
                          ? tr('admin.network.installations.radius_ready_state', 'RADIUS account is ready for live testing.')
                          : tr('admin.network.installations.test_ready_state', 'Router account is ready for live testing.')
                        : installationPppoeTarget === 'managed_radius'
                          ? tr('admin.network.installations.radius_pending_state', 'Create the account first, then test live connectivity through RADIUS.')
                          : tr('admin.network.installations.test_pending_state', 'Create the account first, then test live connectivity from the customer side.')}
                    </span>
                  </div>

                  <div class="modal-actions">
                    {#if !installationPppoeAccount}
                      <button
                        class="btn ghost"
                        type="button"
                        onclick={saveInstallationPppoe}
                        disabled={savingInstallationPppoe || !(installationSubscription?.router_id || activeRow?.router_id || installationPppoeMapping?.router_id) || !installationPppoeMapping?.router_profile_name || !installationPppoeUsername.trim() || !installationPppoePassword || (installationPppoeTarget === 'managed_radius' && !installationManagedRadiusSetup?.configured)}
                      >
                        {savingInstallationPppoe
                          ? tr('common.loading', 'Loading...')
                          : installationPppoeTarget === 'managed_radius'
                            ? tr('admin.network.installations.create_apply_radius', 'Create & Apply to RADIUS')
                            : tr('admin.network.installations.create_and_test', 'Create & Test Connection')}
                      </button>
                    {:else}
                      <button
                        class="btn ghost"
                        type="button"
                        onclick={saveInstallationPppoe}
                        disabled={savingInstallationPppoe || !installationPppoeUsername.trim() || (installationPppoeTarget === 'managed_radius' && !installationManagedRadiusSetup?.configured)}
                      >
                        {savingInstallationPppoe
                          ? tr('common.loading', 'Loading...')
                          : installationPppoeTarget === 'managed_radius'
                            ? tr('admin.network.installations.save_reapply_radius', 'Save & Re-apply to RADIUS')
                            : tr('admin.network.installations.save_reapply_router', 'Save & Re-apply to Router')}
                      </button>
                      <button class="btn ghost" type="button" onclick={applyInstallationPppoe} disabled={savingInstallationPppoe}>
                        {savingInstallationPppoe
                          ? tr('common.loading', 'Loading...')
                          : installationPppoeAccount.account_source === 'managed_radius'
                            ? tr('admin.network.installations.apply_existing_radius', 'Apply Existing to RADIUS')
                            : tr('admin.network.installations.apply_test', 'Apply Test to Router')}
                      </button>
                    {/if}
                  </div>
                {/if}
              </section>
            {/if}
            <fieldset class="checklist single-step">
              <legend>{tr('admin.network.installations.current_step', 'Current Step')}<span class="progress-inline">{onsiteActiveIndex + 1}/{checklistTotal}</span></legend>
              <label class="check-item" class:is-done={getOnsiteTaskChecked(onsiteActiveIndex)}>
                <input
                  type="checkbox"
                  checked={getOnsiteTaskChecked(onsiteActiveIndex)}
                  onchange={(e) => setOnsiteTaskChecked(onsiteActiveIndex, (e.currentTarget as HTMLInputElement).checked)}
                />
                <span class="check-indicator">{getOnsiteTaskChecked(onsiteActiveIndex) ? '✓' : ''}</span>
                <span class="check-content">
                  <strong>{onsiteActiveTask.title}</strong>
                  <small>{onsiteActiveTask.desc}</small>
                </span>
              </label>
            </fieldset>

            <section class="photos-card">
              <div class="photos-head">
                <strong>{tr('admin.network.installations.photos_title', 'Installation Photos')}</strong>
                <label class="btn ghost upload-btn">
                  <Icon name="image" size={14} />
                  {uploadingPhotos ? tr('common.loading', 'Loading...') : tr('admin.network.installations.photos_add', 'Add Photos')}
                  <input type="file" accept="image/*" multiple onchange={uploadInstallationPhotos} disabled={uploadingPhotos} />
                </label>
              </div>

              {#if installationPhotos.length > 0}
                <div class="photo-grid">
                  {#each installationPhotos as file}
                    <article class="photo-item">
                      <a href={getStorageContentUrl(file.id)} target="_blank" rel="noreferrer">
                        <img src={getStorageContentUrl(file.id)} alt={file.original_name || file.name || 'Installation photo'} loading="lazy" />
                      </a>
                      <div class="photo-meta">
                        <span title={file.original_name || file.name || file.id}>{file.original_name || file.name || file.id}</span>
                        <button class="btn danger mini" type="button" onclick={() => removeInstallationPhoto(file.id)}>{tr('common.remove', 'Remove')}</button>
                      </div>
                    </article>
                  {/each}
                </div>
              {:else}
                <p class="helper-text">{tr('admin.network.installations.photos_empty', 'No installation photos uploaded yet.')}</p>
              {/if}
            </section>
            <label class="notes">
              {tr('common.notes', 'Notes')}
              <textarea rows="4" bind:value={formNotes} placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}></textarea>
            </label>
            <div class="modal-actions stage-actions">
              <button class="btn ghost" type="button" onclick={goPrevOnsiteStep} disabled={onsiteActiveIndex === 0}>{tr('common.previous', 'Previous')}</button>
              <button class="btn ghost" type="button" onclick={goNextOnsiteStep} disabled={onsiteActiveIndex >= checklistTotal - 1}>{tr('common.next', 'Next')}</button>
              <button class="btn" type="button" onclick={markActiveOnsiteStepDone} disabled={getOnsiteTaskChecked(onsiteActiveIndex)}>{tr('admin.network.installations.mark_done', 'Mark done')}</button>
              <button class="btn ghost" onclick={savePlan} disabled={busyId === activeRow.id}>{tr('admin.network.installations.save_plan', 'Save Plan')}</button>
            </div>
          {:else if activeRow.status === 'in_progress' && effectiveStep === 4}
            <h3>{tr('admin.network.installations.step_activate', 'Finish')}</h3>
            <p class="step-help">{tr('admin.network.installations.step_active_help', 'Checklist complete. Finish installation to start the service state flow.')}</p>
            <div class="activation-ready">
              <div>{tr('admin.network.installations.checklist', 'Installation Checklist')}: <strong>{checklistDoneCount}/{checklistTotal}</strong></div>
              <div>{tr('common.schedule', 'Schedule')}: <strong>{activeRow.scheduled_at ? formatDateTime(activeRow.scheduled_at) : '-'}</strong></div>
            </div>
            <label class="notes">
              {tr('common.notes', 'Notes')}
              <textarea rows="4" bind:value={formNotes} placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}></textarea>
            </label>
            <div class="modal-actions stage-actions">
              <button class="btn success" onclick={completeFromDetail} disabled={busyId === activeRow.id || !canCompleteActive}>{tr('common.complete', 'Complete')}</button>
            </div>
          {:else if isClosedState}
            <h3>{tr('admin.network.installations.final_state', 'Final State')}</h3>
            <p class="step-help">
              {activeRow.status === 'completed'
                ? isGraceActive
                  ? tr('admin.network.installations.final_grace_active', 'Installation is complete. Service is temporarily active during grace period.')
                  : isAwaitingFirstPayment
                    ? activeRow.has_customer_package_invoice
                      ? tr('admin.network.installations.final_waiting_payment_invoice_exists', 'Installation is complete. First invoice already exists and service is waiting payment before activation.')
                      : tr('admin.network.installations.final_waiting_payment', 'Installation is complete. Service is waiting first payment before activation.')
                    : tr('admin.network.installations.final_completed', 'Installation has been completed and service is active.')
                : tr('admin.network.installations.final_cancelled', 'Installation has been cancelled.')}
            </p>
            {#if activeRow.status === 'completed' && isGraceActive}
              <div class="activation-ready">
                <div>{tr('admin.network.installations.grace_deadline', 'Grace active until')}: <strong>{subscriptionGraceDeadlineLabel}</strong></div>
                <div>{tr('admin.network.installations.grace_followup', 'If the first invoice is still unpaid after this deadline, service will be suspended automatically.')}</div>
              </div>
            {/if}
            {#if canCreateMissingInvoice}
              <div class="modal-actions stage-actions">
                <button class="btn ghost" type="button" onclick={createInvoiceFromDetail} disabled={creatingInvoiceId === activeRow.id}>
                  <Icon name="file-plus" size={14} />
                  {creatingInvoiceId === activeRow.id ? tr('common.loading', 'Loading...') : tr('admin.network.installations.create_invoice', 'Create payment invoice')}
                </button>
              </div>
            {/if}
            {#if activeRow.status === 'cancelled'}
              <label class="notes">
                {tr('common.notes', 'Notes')}
                <textarea rows="3" bind:value={formNotes} placeholder={tr('admin.network.installations.reopen_notes', 'Optional note before reopening work order')}></textarea>
              </label>
              <div class="modal-actions">
                <button class="btn ghost" onclick={() => activeRow && setStatus(activeRow, 'reopen', formNotes)} disabled={busyId === activeRow.id}>{tr('common.reopen', 'Reopen')}</button>
              </div>
            {/if}
          {/if}
        </section>
      {/if}

      {#if canReadAuditLogs}
        <div class="history">
          <h3>{tr('admin.network.installations.timeline', 'Work Order Timeline')}</h3>
          {#if timelineLoading}
            <p class="helper-text">{tr('common.loading', 'Loading...')}</p>
          {:else if timelineRows.length === 0}
            <p class="helper-text">{tr('common.no_data', 'No data')}</p>
          {:else}
            <div class="timeline-list">
              {#each timelineRows as log}
                <article class="timeline-item">
                  <div class="timeline-head">
                    <strong>{log.action}</strong>
                    <span>{formatDateTime(log.created_at)}</span>
                  </div>
                  <div class="timeline-meta">
                    <span>{log.user_name || log.user_email || log.user_id || '-'}</span>
                    {#if log.ip_address}<span>{log.ip_address}</span>{/if}
                  </div>
                  {#if log.details}<p>{log.details}</p>{/if}
                </article>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

{#if cancelDialogOpen && cancelTarget}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    onclick={(e) => {
      if (e.target === e.currentTarget) closeCancelDialog();
    }}
    onkeydown={(e) => {
      if (e.key === 'Escape') closeCancelDialog();
    }}
  >
    <div class="modal cancel-modal">
      <div class="modal-head">
        <h2>{tr('common.cancel', 'Cancel')} Work Order</h2>
        <button class="btn ghost" onclick={closeCancelDialog}>✕</button>
      </div>
      <p class="step-help">{tr('admin.network.installations.cancel_reason_required', 'Cancellation reason is required (minimum 10 characters).')}</p>
      <div class="meta-grid">
        <div><strong>{tr('common.customer', 'Customer')}:</strong> {cancelTarget.customer_name || cancelTarget.customer_id}</div>
        <div><strong>{tr('common.location', 'Location')}:</strong> {cancelTarget.location_label || cancelTarget.location_id}</div>
      </div>
      <label class="notes">
        {tr('common.notes', 'Notes')}
        <textarea rows="4" bind:value={cancelReason} placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}></textarea>
      </label>
      <div class="modal-actions">
        <button class="btn ghost" onclick={closeCancelDialog} disabled={busyId === cancelTarget.id}>{tr('common.close', 'Close')}</button>
        <button class="btn danger" onclick={confirmCancelFromDialog} disabled={busyId === cancelTarget.id || !hasValidCancelReason(cancelReason)}>{tr('common.cancel', 'Cancel')}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(3, 8, 20, 0.66);
    display: grid;
    place-items: center;
    padding: 20px;
    z-index: 1000;
  }
  .modal {
    width: min(900px, 100%);
    max-height: calc(100vh - 40px);
    overflow: auto;
    border-radius: 14px;
    background: #0b1221;
    border: 1px solid #283149;
    padding: 16px;
    display: grid;
    gap: 14px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }
  .cancel-modal {
    width: min(640px, 100%);
  }
  .modal-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    position: sticky;
    top: -16px;
    z-index: 5;
    margin: -16px -16px 0;
    padding: 16px;
    background: rgba(11, 18, 33, 0.94);
    backdrop-filter: blur(10px);
    border-bottom: 1px solid rgba(51, 65, 85, 0.72);
  }
  .modal h2 {
    margin: 0;
    font-size: 1.2rem;
  }
  .btn {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--color-primary);
    color: white;
    font-weight: 800;
    padding: 8px 12px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .btn.mini {
    padding: 5px 9px;
    font-size: 0.76rem;
    border-radius: 10px;
  }
  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }
  .btn.success {
    border-color: rgba(34, 197, 94, 0.28);
    background: rgba(34, 197, 94, 0.14);
    color: rgba(34, 197, 94, 1);
  }
  .btn.danger {
    border-color: rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.14);
    color: rgba(239, 68, 68, 1);
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .status {
    display: inline-flex;
    border-radius: 999px;
    border: 1px solid #374157;
    padding: 2px 10px;
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .status.pending { border-color: #6a5a2b; color: #f6c65f; }
  .status.progress { border-color: #2f5d96; color: #7eb4ff; }
  .status.completed { border-color: #256e43; color: #59d091; }
  .status.cancelled { border-color: #7f2c2c; color: #f18989; }
  .step-flow {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 8px;
  }
  .step-flow > div {
    border: 1px solid #334155;
    border-radius: 16px;
    padding: 10px 12px;
    color: #9fb0cc;
    font-size: 0.82rem;
    text-align: center;
    font-weight: 700;
  }
  .step-flow > div.active-step { border-color: rgba(99, 102, 241, 0.6); background: rgba(99, 102, 241, 0.14); color: #dbeafe; }
  .step-flow > div.done-step { border-color: rgba(34, 197, 94, 0.45); background: rgba(22, 101, 52, 0.22); color: #d1fae5; }
  .meta-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px 14px;
  }
  .focus-panel {
    border: 1px solid rgba(96, 165, 250, 0.28);
    border-radius: 14px;
    background:
      linear-gradient(135deg, rgba(30, 41, 59, 0.96), rgba(15, 23, 42, 0.94)),
      radial-gradient(circle at top right, rgba(59, 130, 246, 0.16), transparent 44%);
    padding: 14px 16px;
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }
  .focus-panel.grace {
    border-color: rgba(34, 197, 94, 0.34);
    background:
      linear-gradient(135deg, rgba(15, 37, 28, 0.96), rgba(11, 18, 33, 0.94)),
      radial-gradient(circle at top right, rgba(34, 197, 94, 0.18), transparent 44%);
  }
  .focus-copy { display: grid; gap: 4px; min-width: 0; }
  .focus-kicker { color: #93c5fd; font-size: 0.72rem; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; }
  .focus-copy strong { color: #eff6ff; font-size: 1rem; }
  .focus-copy p { margin: 0; color: #c6d4ea; font-size: 0.88rem; line-height: 1.45; max-width: 62ch; }
  .focus-chip {
    min-width: 190px; border: 1px solid rgba(148, 163, 184, 0.24); border-radius: 12px;
    background: rgba(15, 23, 42, 0.7); padding: 10px 12px; display: grid; gap: 4px;
  }
  .focus-chip span { color: #93c5fd; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.06em; font-weight: 700; }
  .focus-chip strong { color: #f8fafc; font-size: 0.95rem; }
  .meta-item { border: 1px solid #2b3854; border-radius: 10px; background: #0f1728; padding: 10px 12px; display: grid; gap: 5px; }
  .meta-label { color: #9fb0cc; font-size: 0.75rem; letter-spacing: 0.04em; text-transform: uppercase; font-weight: 700; }
  .meta-value { color: #e5edff; font-size: 0.96rem; font-weight: 800; min-height: 20px; }
  .wizard-card { border: 1px solid #2b3a5b; border-radius: 12px; background: #0e1729; padding: 16px; display: grid; gap: 12px; }
  .wizard-card h3 { margin: 0; font-size: 1rem; }
  .assigned-summary { border: 1px solid #334766; border-radius: 10px; padding: 10px 12px; display: flex; align-items: center; gap: 10px; flex-wrap: wrap; background: #0b1221; }
  .reschedule-request-card { border: 1px solid rgba(245, 158, 11, 0.38); border-radius: 10px; background: rgba(120, 53, 15, 0.18); padding: 12px; display: grid; gap: 10px; }
  .reschedule-request-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; font-size: 0.84rem; color: #fbbf24; }
  .reschedule-request-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px 12px; }
  .reschedule-request-grid > div { display: grid; gap: 4px; }
  .reschedule-request-grid span { font-size: 0.75rem; color: #fcd34d; text-transform: uppercase; letter-spacing: 0.04em; font-weight: 700; }
  .reschedule-request-grid strong { color: #fde68a; font-size: 0.92rem; }
  .reschedule-request-card p { margin: 0; font-size: 0.86rem; color: #fde68a; white-space: pre-wrap; word-break: break-word; }
  .reschedule-decision-fields { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
  .reschedule-decision-fields label:last-child { grid-column: 1 / -1; }
  .summary-label { color: #9fb0cc; font-size: 0.8rem; letter-spacing: 0.03em; text-transform: uppercase; font-weight: 700; }
  .step-help { margin: 0; font-size: 0.9rem; color: #9fb0cc; }
  .form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
  label { display: grid; gap: 6px; font-size: 0.92rem; }
  input[type='datetime-local'], textarea, .input {
    background: #0f1626; color: var(--text, #e9efff); border: 1px solid #2d3650; border-radius: 8px; padding: 8px;
  }
  .checklist { border: 1px solid #2d3650; border-radius: 10px; padding: 10px; display: grid; gap: 8px; }
  .checklist.single-step { padding: 12px; }
  .progress-inline { margin-left: 8px; font-size: 0.78rem; color: #93c5fd; font-weight: 700; }
  .activation-ready { border: 1px dashed #3b5276; border-radius: 12px; padding: 12px; display: grid; gap: 8px; color: #cfe0ff; font-size: 0.9rem; background: rgba(15, 23, 42, 0.52); }
  .check-item { border: 1px solid #314261; background: #0f1728; border-radius: 10px; padding: 10px; cursor: pointer; gap: 10px !important; align-items: flex-start !important; transition: border-color 140ms ease, background 140ms ease; display: flex; }
  .check-item:hover { border-color: #47608d; background: #111d33; }
  .check-item input[type='checkbox'] { position: absolute; opacity: 0; width: 1px; height: 1px; pointer-events: none; }
  .check-indicator { width: 22px; height: 22px; border-radius: 7px; border: 1px solid #496087; background: #0c1422; display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0; color: #0b1a32; font-weight: 900; line-height: 1; }
  .check-content { display: grid; gap: 3px; color: #d9e7ff; }
  .check-content strong { font-size: 0.96rem; }
  .check-content small { color: #9eb0cf; font-size: 0.8rem; }
  .check-item.is-done { border-color: rgba(34, 197, 94, 0.44); background: rgba(22, 101, 52, 0.2); }
  .check-item.is-done .check-indicator { border-color: rgba(34, 197, 94, 0.65); background: #22c55e; color: #06280f; }
  .check-item.is-done .check-content strong { color: #d1fadf; }
  .cable-designer-card { border: 1px solid #2d3f61; border-radius: 12px; background: linear-gradient(135deg, #0c162a, #101c31); padding: 12px; display: flex; justify-content: space-between; align-items: center; gap: 10px; }
  .pppoe-install-card { border: 1px solid rgba(59, 130, 246, 0.26); border-radius: 14px; background: linear-gradient(180deg, rgba(11, 23, 41, 0.96), rgba(12, 18, 33, 0.98)), radial-gradient(circle at top right, rgba(59, 130, 246, 0.14), transparent 45%); padding: 14px; display: grid; gap: 12px; }
  .pppoe-install-head { display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; }
  .pppoe-install-head p { margin: 4px 0 0; color: #b7c8e7; font-size: 0.86rem; max-width: 58ch; }
  .pppoe-existing { border: 1px solid rgba(34, 197, 94, 0.22); border-radius: 12px; background: rgba(21, 128, 61, 0.12); padding: 10px 12px; display: flex; gap: 8px; align-items: center; flex-wrap: wrap; color: #d6f5e3; }
  .pppoe-existing span:first-child { color: #9fd7b2; font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 700; }
  .test-outcome { display: flex; justify-content: flex-start; }
  .test-state { border: 1px dashed rgba(148, 163, 184, 0.34); border-radius: 999px; padding: 7px 12px; color: #c7d3e7; font-size: 0.82rem; background: rgba(15, 23, 42, 0.45); }
  .test-state.ok { border-color: rgba(34, 197, 94, 0.38); color: #d4f7df; background: rgba(22, 101, 52, 0.2); }
  .cable-designer-copy { display: grid; gap: 4px; }
  .cable-designer-copy p { margin: 0; font-size: 0.85rem; color: #9fb0cc; }
  .cable-map-drawer { margin-top: 10px; border: 1px solid #2d3f61; border-radius: 10px; background: #0a1220; overflow: hidden; }
  .cable-map-head { display: flex; justify-content: space-between; align-items: center; gap: 10px; padding: 8px 10px; border-bottom: 1px solid #263655; background: #0b1629; }
  .cable-map-drawer :global(.icm-map) { border-top-left-radius: 0; border-top-right-radius: 0; border-left: 0; border-right: 0; border-bottom: 0; }
  .photos-card { border: 1px solid #2d3650; border-radius: 10px; padding: 10px; display: grid; gap: 10px; background: #0f1626; }
  .photos-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .upload-btn { position: relative; overflow: hidden; }
  .upload-btn input[type='file'] { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
  .photo-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(130px, 1fr)); gap: 8px; }
  .photo-item { border: 1px solid #2d3650; border-radius: 10px; background: #0b1221; overflow: hidden; display: grid; gap: 6px; padding: 6px; }
  .photo-item img { width: 100%; height: 92px; object-fit: cover; border-radius: 6px; border: 1px solid #2d3650; display: block; background: #0a1220; }
  .photo-meta { display: flex; align-items: center; justify-content: space-between; gap: 6px; }
  .photo-meta span { min-width: 0; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.75rem; color: #b8c7e3; }
  .notes textarea { resize: vertical; min-height: 110px; }
  .helper-text { margin: 0; color: var(--text-secondary); font-size: 0.85rem; }
  .modal-actions { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .stage-actions { position: sticky; bottom: -16px; z-index: 4; margin: 6px -16px -16px; padding: 14px 16px 16px; background: linear-gradient(180deg, rgba(11, 18, 33, 0), rgba(11, 18, 33, 0.92) 22%, rgba(11, 18, 33, 0.98)); backdrop-filter: blur(8px); border-top: 1px solid rgba(51, 65, 85, 0.72); }
  .history { border-top: 1px dashed #33405d; padding-top: 10px; }
  .history h3 { margin: 0 0 8px; font-size: 0.95rem; }
  .timeline-list { display: grid; gap: 8px; }
  .timeline-item { border: 1px solid #2d3650; border-radius: 10px; padding: 10px; background: #0f1626; display: grid; gap: 4px; }
  .timeline-head { display: flex; justify-content: space-between; gap: 10px; align-items: center; }
  .timeline-head strong { font-size: 0.9rem; }
  .timeline-head span, .timeline-meta { color: #9fb0cc; font-size: 0.78rem; }
  .timeline-meta { display: flex; gap: 10px; }
  .timeline-item p { margin: 0; color: #c9d6ef; font-size: 0.85rem; white-space: pre-wrap; word-break: break-word; }
  @media (max-width: 800px) {
    .meta-grid, .form-grid, .reschedule-request-grid, .reschedule-decision-fields { grid-template-columns: 1fr; }
    .focus-panel, .pppoe-install-head, .cable-designer-card { grid-template-columns: 1fr; display: grid; }
    .step-flow { grid-template-columns: 1fr; }
    .step-flow > div { text-align: left; }
    .modal-head { top: -16px; align-items: flex-start; }
    .modal-head .btn { flex-shrink: 0; }
    .stage-actions { justify-content: stretch; }
    .stage-actions .btn { flex: 1 1 100%; justify-content: center; }
  }
</style>
