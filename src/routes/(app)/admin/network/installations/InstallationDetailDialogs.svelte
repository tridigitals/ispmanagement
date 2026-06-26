<script lang="ts">
  import { tick } from 'svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import InstallationCableMap from '$lib/components/network/InstallationCableMap.svelte';
  import { formatDhcpStaticMacAddressInput } from '$lib/utils/dhcpStaticValidation';

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
    loadingInstallationDhcp,
    installationSubscription,
    installationPppoeUsername = $bindable(),
    installationPppoePassword = $bindable(),
    installationPppoeComment = $bindable(),
    installationPppoeTarget = $bindable(),
    installationDhcpServerName = $bindable(),
    installationDhcpServerNameError = $bindable(),
    installationDhcpRouterError = $bindable(),
    installationDhcpMacAddress = $bindable(),
    installationDhcpMacAddressError = $bindable(),
    installationDhcpIpAddress = $bindable(),
    installationDhcpIpAddressError = $bindable(),
    installationDhcpComment = $bindable(),
    installationDhcpQueueMode = $bindable(),
    installationDhcpQueueRateLimit = $bindable(),
    installationDhcpQueueRateLimitError = $bindable(),
    installationDhcpQueueRateLimitPresets = [],
    loadingInstallationAssets,
    savingInstallationAssets,
    installationTerminalAssetId = $bindable(),
    installationParentAssetId = $bindable(),
    installationTerminalAssetOptions,
    installationParentAssetOptions,
    installationAssetBindingError,
    selectedTerminalAssetLabel,
    selectedParentAssetLabel,
    handleInstallationTerminalAssetChange,
    handleInstallationParentAssetChange,
    installationQuickAssetOpen = $bindable(),
    creatingInstallationQuickAsset,
    installationQuickAssetDraft = $bindable(),
    installationQuickAssetDuplicates = {},
    installationQuickAssetCanSubmit = true,
    openInstallationQuickAsset,
    closeInstallationQuickAsset,
    createInstallationQuickAsset,
    updateInstallationQuickAssetField,
    continueToFinishStep,
    installationPppoeTargetOptions,
    installationManagedRadiusHint,
    installationManagedRadiusLoadError,
    installationManagedRadiusSetup,
    installationPppoeAccount,
    installationDhcpService,
    savingInstallationPppoe,
    savingInstallationDhcp,
    saveInstallationPppoe,
    applyInstallationPppoe,
    saveInstallationDhcp,
    applyInstallationDhcp,
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

  let cancelReasonTextarea = $state<HTMLTextAreaElement | null>(null);

  $effect(() => {
    if (!cancelDialogOpen || !cancelTarget) return;
    void tick().then(() => {
      cancelReasonTextarea?.focus();
      cancelReasonTextarea?.select();
    });
  });
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
        <button class="btn ghost icon-btn" onclick={closeDetail} aria-label={tr('common.close', 'Close')}>
          <Icon name="x" size={16} />
        </button>
      </div>

      <div class="step-flow">
        {#if activeRow.status === 'in_progress'}
          <div class:active-step={true}>1. {tr('admin.network.installations.step_assign', 'Assign')}</div>
          <div class:active-step={true}>2. {tr('admin.network.installations.step_schedule', 'Schedule')}</div>
          <div class:done-step={checkCable} class:active-step={!checkCable && onsiteActiveIndex === 0}>3. Cable</div>
          <div class:done-step={checkOnt} class:active-step={!checkOnt && onsiteActiveIndex === 1}>4. ONT</div>
          <div class:done-step={checkPppoe} class:active-step={!checkPppoe && onsiteActiveIndex === 2}>5. PPPoE</div>
          <div class:done-step={checkSpeed} class:active-step={!checkSpeed && onsiteActiveIndex === 3}>6. Speed Test</div>
          <div class:active-step={effectiveStep >= 4}>7. {tr('admin.network.installations.step_assets', 'Asset Binding')}</div>
          <div class:active-step={effectiveStep >= 5}>8. {tr('admin.network.installations.step_activate', 'Activate')}</div>
        {:else}
          <div class:active-step={effectiveStep >= 1}>1. {tr('admin.network.installations.step_assign', 'Assign')}</div>
          <div class:active-step={effectiveStep >= 2}>2. {tr('admin.network.installations.step_schedule', 'Schedule')}</div>
          <div class:active-step={effectiveStep >= 3}>3. {tr('admin.network.installations.step_onsite', 'On-site & Test')}</div>
          <div class:active-step={effectiveStep >= 4}>4. {tr('admin.network.installations.step_assets', 'Asset Binding')}</div>
          <div class:active-step={effectiveStep >= 5}>5. {tr('admin.network.installations.step_activate', 'Finish')}</div>
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
                    <p>
                      {activeRow?.package_provisioning_type === 'dhcp_static'
                        ? tr(
                            'admin.network.installations.internet_test_help_dhcp',
                            'Technician enters DHCP server, MAC address, and static IP for the customer device.',
                          )
                        : tr(
                            'admin.network.installations.internet_test_help',
                            'Technician only enters username and password. Router, profile, and pool follow the active internet package mapping.',
                          )}
                    </p>
                  </div>
                  {#if activeRow?.package_provisioning_type === 'dhcp_static' ? installationDhcpService : installationPppoeAccount}
                    <span class="status progress">{tr('admin.network.installations.internet_test_configured', 'Configured')}</span>
                  {/if}
                </div>

                {#if activeRow?.package_provisioning_type === 'dhcp_static' ? loadingInstallationDhcp : loadingInstallationPppoe}
                  <p class="helper-text">{tr('common.loading', 'Loading...')}</p>
                {:else if !installationSubscription && !activeRow?.package_id && !(activeRow?.package_provisioning_type === 'dhcp_static' ? installationDhcpService?.package_id : installationPppoeAccount?.package_id)}
                  <p class="helper-text">{tr('admin.network.installations.subscription_not_found', 'Subscription internet untuk work order ini belum ditemukan.')}</p>
                {:else if activeRow?.package_provisioning_type === 'dhcp_static'}
                  <div class="form-grid two-col compact">
                    <label class="summary-field">
                      {tr('common.router', 'Router')}
                      <input
                        class:error={!!installationDhcpRouterError}
                        class="input"
                        value={activeRow?.router_name || installationSubscription?.router_name || '-'}
                        disabled
                      />
                      {#if installationDhcpRouterError}
                        <span class="field-error">{installationDhcpRouterError}</span>
                      {/if}
                    </label>
                    <label class="summary-field">
                      {tr('admin.network.installations.dhcp_server', 'DHCP Server')}
                      <input
                        class:error={!!installationDhcpServerNameError}
                        class="input"
                        bind:value={installationDhcpServerName}
                        placeholder="dhcp server name"
                      />
                      {#if installationDhcpServerNameError}
                        <span class="field-error">{installationDhcpServerNameError}</span>
                      {/if}
                    </label>
                    <label class="summary-field">
                      {tr('admin.network.installations.mac_address', 'MAC Address')}
                      <input
                        class:error={!!installationDhcpMacAddressError}
                        class="input"
                        value={installationDhcpMacAddress}
                        oninput={(event) =>
                          (installationDhcpMacAddress = formatDhcpStaticMacAddressInput(
                            (event.currentTarget as HTMLInputElement).value,
                          ))}
                        placeholder="AA:BB:CC:DD:EE:FF"
                      />
                      {#if installationDhcpMacAddressError}
                        <span class="field-error">{installationDhcpMacAddressError}</span>
                      {/if}
                    </label>
                    <label class="summary-field">
                      {tr('admin.network.installations.ip_address', 'IP Address')}
                      <input class:error={!!installationDhcpIpAddressError} class="input" bind:value={installationDhcpIpAddress} placeholder="192.168.1.10" />
                      {#if installationDhcpIpAddressError}
                        <span class="field-error">{installationDhcpIpAddressError}</span>
                      {/if}
                    </label>
                    <label class="summary-field">
                      {tr('admin.network.installations.queue_mode', 'Queue Mode')}
                      <select class="input" bind:value={installationDhcpQueueMode}>
                        <option value="none">{tr('admin.network.installations.queue_none', 'No queue')}</option>
                        <option value="simple_queue">{tr('admin.network.installations.queue_simple', 'Simple queue')}</option>
                      </select>
                    </label>
                  </div>
                  {#if installationDhcpQueueMode === 'simple_queue'}
                    <label class="summary-field">
                      {tr('admin.network.installations.queue_rate_limit', 'Queue Rate Limit')}
                      <input class:error={!!installationDhcpQueueRateLimitError} class="input" bind:value={installationDhcpQueueRateLimit} placeholder="20M/20M" />
                      <span class="helper-text">
                        {tr(
                          'admin.network.dhcp_static.fields.queue_rate_limit_hint',
                          'Use format like 20M/20M for download/upload.',
                        )}
                      </span>
                      <div class="preset-group">
                        <span class="preset-label">
                          {tr('admin.network.dhcp_static.fields.queue_presets', 'Quick presets')}
                        </span>
                        <div class="preset-chips">
                          {#each installationDhcpQueueRateLimitPresets as preset}
                            <button
                              type="button"
                              class="preset-chip"
                              onclick={() => (installationDhcpQueueRateLimit = preset)}
                            >
                              {preset}
                            </button>
                          {/each}
                        </div>
                      </div>
                      {#if installationDhcpQueueRateLimitError}
                        <span class="field-error">{installationDhcpQueueRateLimitError}</span>
                      {/if}
                    </label>
                  {/if}
                  <label class="notes">
                    {tr('common.comment', 'Comment')}
                    <input class="input" bind:value={installationDhcpComment} placeholder={tr('admin.network.installations.dhcp_comment_placeholder', 'Optional DHCP lease comment')} />
                  </label>
                  {#if installationDhcpService}
                    <div class="pppoe-existing">
                      <span>{tr('admin.network.installations.existing_dhcp', 'Existing DHCP:')}</span>
                      <strong>{installationDhcpService.mac_address}</strong>
                      <span>{installationDhcpService.ip_address}</span>
                    </div>
                  {/if}
                  <div class="test-outcome">
                    <span class:ok={!!installationDhcpService} class="test-state">
                      {installationDhcpService
                        ? tr('admin.network.installations.dhcp_ready_state', 'DHCP static lease is ready for live testing.')
                        : tr('admin.network.installations.dhcp_pending_state', 'Create the static lease first, then verify connectivity from the customer side.')}
                    </span>
                  </div>
                  <div class="modal-actions">
                    {#if !installationDhcpService}
                      <button
                        class="btn ghost"
                        type="button"
                        onclick={saveInstallationDhcp}
                        disabled={savingInstallationDhcp || !installationDhcpServerName.trim() || !installationDhcpMacAddress.trim() || !installationDhcpIpAddress.trim()}
                      >
                        {savingInstallationDhcp ? tr('common.loading', 'Loading...') : tr('admin.network.installations.create_apply_lease', 'Create & Apply Lease')}
                      </button>
                    {:else}
                      <button
                        class="btn ghost"
                        type="button"
                        onclick={saveInstallationDhcp}
                        disabled={savingInstallationDhcp || !installationDhcpServerName.trim() || !installationDhcpMacAddress.trim() || !installationDhcpIpAddress.trim()}
                      >
                        {savingInstallationDhcp ? tr('common.loading', 'Loading...') : tr('admin.network.installations.save_reapply_lease', 'Save & Re-apply Lease')}
                      </button>
                      <button class="btn ghost" type="button" onclick={applyInstallationDhcp} disabled={savingInstallationDhcp}>
                        {savingInstallationDhcp ? tr('common.loading', 'Loading...') : tr('admin.network.installations.apply_existing_lease', 'Apply Existing Lease')}
                      </button>
                    {/if}
                  </div>
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
                <span class="check-indicator">
                  {#if getOnsiteTaskChecked(onsiteActiveIndex)}
                    <Icon name="check" size={14} />
                  {/if}
                </span>
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
            <h3>{tr('admin.network.installations.step_assets', 'Asset Binding')}</h3>
            <p class="step-help">{tr('admin.network.installations.step_assets_help', 'Select the terminal asset installed on site, then optionally connect the upstream parent asset for audit traceability.')}</p>
            {#if loadingInstallationAssets}
              <p class="helper-text">{tr('common.loading', 'Loading...')}</p>
            {:else}
              <div class="form-grid two-col compact">
                <label class="summary-field">
                  {tr('admin.network.installations.terminal_asset', 'Terminal Asset (ONT/ONU)')}
                  <select
                    class:error={!!installationAssetBindingError}
                    class="input"
                    bind:value={installationTerminalAssetId}
                    onchange={(event) => handleInstallationTerminalAssetChange((event.currentTarget as HTMLSelectElement).value)}
                  >
                    <option value="">{tr('admin.network.installations.select_terminal_asset', 'Select ONT/ONU asset')}</option>
                    {#each installationTerminalAssetOptions as option}
                      <option value={option.value}>{option.label}</option>
                    {/each}
                  </select>
                  {#if installationAssetBindingError}
                    <span class="field-error">{installationAssetBindingError}</span>
                  {/if}
                </label>
                <label class="summary-field">
                  {tr('admin.network.installations.parent_asset_optional', 'Parent Asset (Optional)')}
                  <select
                    class="input"
                    bind:value={installationParentAssetId}
                    onchange={(event) => handleInstallationParentAssetChange((event.currentTarget as HTMLSelectElement).value)}
                  >
                    <option value="">{tr('admin.network.installations.no_parent_asset', 'No parent asset')}</option>
                    {#each installationParentAssetOptions as option}
                      <option value={option.value}>{option.label}</option>
                    {/each}
                  </select>
                </label>
              </div>
              <div class="modal-actions">
                <button class="btn ghost" type="button" onclick={openInstallationQuickAsset}>
                  <Icon name="plus" size={14} />
                  {tr('admin.network.installations.quick_create_terminal_asset', 'Create ONT/ONU')}
                </button>
              </div>
              {#if installationQuickAssetOpen}
                <section class="quick-asset-card">
                  <div class="quick-asset-head">
                    <strong>{tr('admin.network.installations.quick_create_title', 'Quick Create Terminal Asset')}</strong>
                    <button class="btn ghost mini" type="button" onclick={closeInstallationQuickAsset}>
                      {tr('common.close', 'Close')}
                    </button>
                  </div>
                  <div class="form-grid two-col compact">
                    <label class="summary-field">
                      {tr('admin.network.installations.quick_asset_type', 'Type')}
                      <select
                        class="input"
                        value={installationQuickAssetDraft.asset_type}
                        onchange={(event) =>
                          updateInstallationQuickAssetField(
                            'asset_type',
                            (event.currentTarget as HTMLSelectElement).value,
                          )}
                      >
                        <option value="ont">ONT</option>
                        <option value="onu">ONU</option>
                      </select>
                    </label>
                    <label class="summary-field">
                      {tr('admin.network.installations.quick_asset_name', 'Asset Name')}
                      <input
                        class="input"
                        value={installationQuickAssetDraft.name}
                        oninput={(event) =>
                          updateInstallationQuickAssetField(
                            'name',
                            (event.currentTarget as HTMLInputElement).value,
                          )}
                        placeholder="ONT Customer A"
                      />
                    </label>
                    <label class="summary-field">
                      {tr('admin.network.installations.quick_asset_serial', 'Serial Number')}
                      <input
                        class="input"
                        value={installationQuickAssetDraft.serial_number}
                        oninput={(event) =>
                          updateInstallationQuickAssetField(
                            'serial_number',
                            (event.currentTarget as HTMLInputElement).value,
                        )}
                        placeholder="SN-123456"
                      />
                      {#if installationQuickAssetDuplicates.serial_number}
                        <span class="field-error">{installationQuickAssetDuplicates.serial_number}</span>
                      {/if}
                    </label>
                    <label class="summary-field">
                      {tr('admin.network.installations.quick_asset_code', 'Code')}
                      <input
                        class="input"
                        value={installationQuickAssetDraft.code}
                        oninput={(event) =>
                          updateInstallationQuickAssetField(
                            'code',
                            (event.currentTarget as HTMLInputElement).value,
                        )}
                        placeholder="ONT-001"
                      />
                      {#if installationQuickAssetDuplicates.code}
                        <span class="field-error">{installationQuickAssetDuplicates.code}</span>
                      {/if}
                    </label>
                    <label class="summary-field">
                      {tr('admin.network.installations.quick_asset_vendor', 'Vendor')}
                      <input
                        class="input"
                        value={installationQuickAssetDraft.vendor}
                        oninput={(event) =>
                          updateInstallationQuickAssetField(
                            'vendor',
                            (event.currentTarget as HTMLInputElement).value,
                          )}
                        placeholder="ZTE"
                      />
                    </label>
                    <label class="summary-field">
                      {tr('admin.network.installations.quick_asset_model', 'Model')}
                      <input
                        class="input"
                        value={installationQuickAssetDraft.model}
                        oninput={(event) =>
                          updateInstallationQuickAssetField(
                            'model',
                            (event.currentTarget as HTMLInputElement).value,
                          )}
                        placeholder="F670L"
                      />
                    </label>
                  </div>
                  <div class="modal-actions">
                    <button class="btn ghost" type="button" onclick={closeInstallationQuickAsset}>
                      {tr('common.cancel', 'Cancel')}
                    </button>
                    <button
                      class="btn"
                      type="button"
                      onclick={createInstallationQuickAsset}
                      disabled={!installationQuickAssetCanSubmit}
                    >
                      {creatingInstallationQuickAsset
                        ? tr('common.loading', 'Loading...')
                        : tr('admin.network.installations.quick_create_submit', 'Create & Select')}
                    </button>
                  </div>
                </section>
              {/if}
              <div class="activation-ready">
                <div>{tr('admin.network.installations.terminal_asset_selected', 'Terminal asset')}: <strong>{selectedTerminalAssetLabel || '-'}</strong></div>
                <div>{tr('admin.network.installations.parent_asset_selected', 'Parent asset')}: <strong>{selectedParentAssetLabel || '-'}</strong></div>
              </div>
              <div class="modal-actions stage-actions">
                <button
                  class="btn"
                  type="button"
                  onclick={continueToFinishStep}
                  disabled={!installationTerminalAssetId || savingInstallationAssets}
                >
                  {savingInstallationAssets
                    ? tr('common.loading', 'Loading...')
                    : tr('admin.network.installations.continue_finish', 'Continue to Finish')}
                </button>
              </div>
            {/if}
          {:else if activeRow.status === 'in_progress' && effectiveStep === 5}
            <h3>{tr('admin.network.installations.step_activate', 'Finish')}</h3>
            <p class="step-help">{tr('admin.network.installations.step_active_help', 'Checklist complete. Finish installation to start the service state flow.')}</p>
            <div class="activation-ready">
              <div>{tr('admin.network.installations.checklist', 'Installation Checklist')}: <strong>{checklistDoneCount}/{checklistTotal}</strong></div>
              <div>{tr('common.schedule', 'Schedule')}: <strong>{activeRow.scheduled_at ? formatDateTime(activeRow.scheduled_at) : '-'}</strong></div>
              <div>{tr('admin.network.installations.terminal_asset_selected', 'Terminal asset')}: <strong>{selectedTerminalAssetLabel || '-'}</strong></div>
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
            {#if selectedTerminalAssetLabel || selectedParentAssetLabel}
              <div class="activation-ready">
                <div>{tr('admin.network.installations.terminal_asset_selected', 'Terminal asset')}: <strong>{selectedTerminalAssetLabel || '-'}</strong></div>
                <div>{tr('admin.network.installations.parent_asset_selected', 'Parent asset')}: <strong>{selectedParentAssetLabel || '-'}</strong></div>
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
        <button class="btn ghost icon-btn" onclick={closeCancelDialog} aria-label={tr('common.close', 'Close')}>
          <Icon name="x" size={16} />
        </button>
      </div>
      <p class="step-help">
        {tr('admin.network.installations.cancel_reason_required', 'Cancellation reason is required (minimum 10 characters).')}
        {` `}
        {tr('admin.network.installations.cancel_reason_editable', 'You can replace the default reason with a more specific note.')}
      </p>
      <div class="meta-grid">
        <div><strong>{tr('common.customer', 'Customer')}:</strong> {cancelTarget.customer_name || cancelTarget.customer_id}</div>
        <div><strong>{tr('common.location', 'Location')}:</strong> {cancelTarget.location_label || cancelTarget.location_id}</div>
      </div>
      <label class="notes">
        {tr('common.notes', 'Notes')}
        <textarea
          bind:this={cancelReasonTextarea}
          rows="4"
          bind:value={cancelReason}
          placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}
        ></textarea>
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
    background: color-mix(in srgb, var(--bg-app) 72%, transparent);
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
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    padding: 16px;
    display: grid;
    gap: 14px;
    box-shadow: var(--shadow-md);
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
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border-color);
  }
  .modal h2 {
    margin: 0;
    font-size: 1.2rem;
  }
  .btn {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--color-primary);
    color: var(--bg-app);
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
  .icon-btn {
    width: 36px;
    height: 36px;
    padding: 0;
    justify-content: center;
  }
  .btn.success {
    border-color: color-mix(in srgb, var(--color-success) 38%, var(--border-color));
    background: var(--bg-success);
    color: var(--text-success);
  }
  .btn.danger {
    border-color: color-mix(in srgb, var(--color-danger) 34%, var(--border-color));
    background: color-mix(in srgb, var(--color-danger) 14%, transparent);
    color: var(--color-danger);
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .status {
    display: inline-flex;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    padding: 2px 10px;
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .status.pending { border-color: color-mix(in srgb, var(--color-warning) 40%, var(--border-color)); color: var(--color-warning); }
  .status.progress { border-color: color-mix(in srgb, var(--color-primary) 40%, var(--border-color)); color: var(--color-primary); }
  .status.completed { border-color: color-mix(in srgb, var(--color-success) 40%, var(--border-color)); color: var(--text-success); }
  .status.cancelled { border-color: color-mix(in srgb, var(--color-danger) 40%, var(--border-color)); color: var(--color-danger); }
  .step-flow {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 8px;
  }
  .step-flow > div {
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 10px 12px;
    color: var(--text-secondary);
    font-size: 0.82rem;
    text-align: center;
    font-weight: 700;
  }
  .step-flow > div.active-step { border-color: color-mix(in srgb, var(--color-primary) 48%, var(--border-color)); background: var(--color-primary-subtle); color: var(--text-primary); }
  .step-flow > div.done-step { border-color: color-mix(in srgb, var(--color-success) 48%, var(--border-color)); background: var(--bg-success); color: var(--text-primary); }
  .meta-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px 14px;
  }
  .focus-panel {
    border: 1px solid color-mix(in srgb, var(--color-primary) 32%, var(--border-color));
    border-radius: 14px;
    background: var(--bg-surface);
    padding: 14px 16px;
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }
  .focus-panel.grace {
    border-color: color-mix(in srgb, var(--color-success) 34%, var(--border-color));
    background: var(--bg-surface);
  }
  .focus-copy { display: grid; gap: 4px; min-width: 0; }
  .focus-kicker { color: var(--color-primary); font-size: 0.72rem; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; }
  .focus-copy strong { color: var(--text-primary); font-size: 1rem; }
  .focus-copy p { margin: 0; color: var(--text-secondary); font-size: 0.88rem; line-height: 1.45; max-width: 62ch; }
  .focus-chip {
    min-width: 190px; border: 1px solid var(--border-color); border-radius: 12px;
    background: var(--bg-primary); padding: 10px 12px; display: grid; gap: 4px;
  }
  .focus-chip span { color: var(--color-primary); font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.06em; font-weight: 700; }
  .focus-chip strong { color: var(--text-primary); font-size: 0.95rem; }
  .meta-item { border: 1px solid var(--border-color); border-radius: 10px; background: var(--bg-surface); padding: 10px 12px; display: grid; gap: 5px; }
  .meta-label { color: var(--text-secondary); font-size: 0.75rem; letter-spacing: 0.04em; text-transform: uppercase; font-weight: 700; }
  .meta-value { color: var(--text-primary); font-size: 0.96rem; font-weight: 800; min-height: 20px; }
  .wizard-card { border: 1px solid var(--border-color); border-radius: 12px; background: var(--bg-surface); padding: 16px; display: grid; gap: 12px; }
  .wizard-card h3 { margin: 0; font-size: 1rem; }
  .assigned-summary { border: 1px solid var(--border-color); border-radius: 10px; padding: 10px 12px; display: flex; align-items: center; gap: 10px; flex-wrap: wrap; background: var(--bg-primary); }
  .reschedule-request-card { border: 1px solid color-mix(in srgb, var(--color-warning) 42%, var(--border-color)); border-radius: 10px; background: color-mix(in srgb, var(--color-warning) 12%, transparent); padding: 12px; display: grid; gap: 10px; }
  .reschedule-request-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; font-size: 0.84rem; color: var(--color-warning); }
  .reschedule-request-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px 12px; }
  .reschedule-request-grid > div { display: grid; gap: 4px; }
  .reschedule-request-grid span { font-size: 0.75rem; color: var(--color-warning); text-transform: uppercase; letter-spacing: 0.04em; font-weight: 700; }
  .reschedule-request-grid strong { color: var(--text-primary); font-size: 0.92rem; }
  .reschedule-request-card p { margin: 0; font-size: 0.86rem; color: var(--text-primary); white-space: pre-wrap; word-break: break-word; }
  .reschedule-decision-fields { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
  .reschedule-decision-fields label:last-child { grid-column: 1 / -1; }
  .summary-label { color: var(--text-secondary); font-size: 0.8rem; letter-spacing: 0.03em; text-transform: uppercase; font-weight: 700; }
  .step-help { margin: 0; font-size: 0.9rem; color: var(--text-secondary); }
  .form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
  label { display: grid; gap: 6px; font-size: 0.92rem; }
  input[type='datetime-local'], textarea, .input {
    background: var(--bg-primary); color: var(--text-primary); border: 1px solid var(--border-color); border-radius: 8px; padding: 8px;
  }
  .input.error {
    border-color: var(--color-danger-500, #ef4444);
  }
  .checklist { border: 1px solid var(--border-color); border-radius: 10px; padding: 10px; display: grid; gap: 8px; }
  .checklist.single-step { padding: 12px; }
  .progress-inline { margin-left: 8px; font-size: 0.78rem; color: var(--color-primary); font-weight: 700; }
  .activation-ready { border: 1px dashed var(--border-color); border-radius: 12px; padding: 12px; display: grid; gap: 8px; color: var(--text-primary); font-size: 0.9rem; background: var(--bg-surface); }
  .quick-asset-card { border: 1px solid var(--border-color); border-radius: 12px; background: var(--bg-surface); padding: 12px; display: grid; gap: 12px; }
  .quick-asset-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .check-item { border: 1px solid var(--border-color); background: var(--bg-surface); border-radius: 10px; padding: 10px; cursor: pointer; gap: 10px !important; align-items: flex-start !important; transition: border-color 140ms ease, background 140ms ease; display: flex; }
  .check-item:hover { border-color: var(--color-primary); background: var(--bg-hover); }
  .check-item input[type='checkbox'] { position: absolute; opacity: 0; width: 1px; height: 1px; pointer-events: none; }
  .check-indicator { width: 22px; height: 22px; border-radius: 7px; border: 1px solid var(--border-color); background: var(--bg-primary); display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0; color: var(--text-primary); font-weight: 900; line-height: 1; }
  .check-content { display: grid; gap: 3px; color: var(--text-primary); }
  .check-content strong { font-size: 0.96rem; }
  .check-content small { color: var(--text-secondary); font-size: 0.8rem; }
  .check-item.is-done { border-color: var(--color-success); background: var(--bg-success); }
  .check-item.is-done .check-indicator { border-color: var(--color-success); background: var(--color-success); color: var(--bg-app); }
  .check-item.is-done .check-content strong { color: var(--text-primary); }
  .cable-designer-card { border: 1px solid var(--border-color); border-radius: 12px; background: var(--bg-surface); padding: 12px; display: flex; justify-content: space-between; align-items: center; gap: 10px; }
  .pppoe-install-card { border: 1px solid var(--border-color); border-radius: 14px; background: var(--bg-surface); padding: 14px; display: grid; gap: 12px; }
  .pppoe-install-head { display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; }
  .pppoe-install-head p { margin: 4px 0 0; color: var(--text-secondary); font-size: 0.86rem; max-width: 58ch; }
  .pppoe-existing { border: 1px solid color-mix(in srgb, var(--color-success) 28%, var(--border-color)); border-radius: 12px; background: var(--bg-success); padding: 10px 12px; display: flex; gap: 8px; align-items: center; flex-wrap: wrap; color: var(--text-primary); }
  .pppoe-existing span:first-child { color: var(--text-success); font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 700; }
  .test-outcome { display: flex; justify-content: flex-start; }
  .test-state { border: 1px dashed var(--border-color); border-radius: 999px; padding: 7px 12px; color: var(--text-secondary); font-size: 0.82rem; background: var(--bg-primary); }
  .test-state.ok { border-color: color-mix(in srgb, var(--color-success) 38%, var(--border-color)); color: var(--text-success); background: var(--bg-success); }
  .cable-designer-copy { display: grid; gap: 4px; }
  .cable-designer-copy p { margin: 0; font-size: 0.85rem; color: var(--text-secondary); }
  .cable-map-drawer { margin-top: 10px; border: 1px solid var(--border-color); border-radius: 10px; background: var(--bg-primary); overflow: hidden; }
  .cable-map-head { display: flex; justify-content: space-between; align-items: center; gap: 10px; padding: 8px 10px; border-bottom: 1px solid var(--border-color); background: var(--bg-surface); }
  .cable-map-drawer :global(.icm-map) { border-top-left-radius: 0; border-top-right-radius: 0; border-left: 0; border-right: 0; border-bottom: 0; }
  .photos-card { border: 1px solid var(--border-color); border-radius: 10px; padding: 10px; display: grid; gap: 10px; background: var(--bg-primary); }
  .photos-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .upload-btn { position: relative; overflow: hidden; }
  .upload-btn input[type='file'] { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
  .photo-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(130px, 1fr)); gap: 8px; }
  .photo-item { border: 1px solid var(--border-color); border-radius: 10px; background: var(--bg-surface); overflow: hidden; display: grid; gap: 6px; padding: 6px; }
  .photo-item img { width: 100%; height: 92px; object-fit: cover; border-radius: 6px; border: 1px solid var(--border-color); display: block; background: var(--bg-primary); }
  .photo-meta { display: flex; align-items: center; justify-content: space-between; gap: 6px; }
  .photo-meta span { min-width: 0; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.75rem; color: var(--text-secondary); }
  .notes textarea { resize: vertical; min-height: 110px; }
  .helper-text { margin: 0; color: var(--text-secondary); font-size: 0.85rem; }
  .field-error { margin: 0; color: var(--color-danger-700, #b91c1c); font-size: 0.82rem; }
  .preset-group { display: grid; gap: 6px; }
  .preset-label { font-size: 0.78rem; color: var(--text-secondary); }
  .preset-chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .preset-chip {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 4px 10px;
    font-size: 0.78rem;
    cursor: pointer;
  }
  .modal-actions { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .stage-actions { position: sticky; bottom: -16px; z-index: 4; margin: 6px -16px -16px; padding: 14px 16px 16px; background: var(--bg-surface); border-top: 1px solid var(--border-color); }
  .history { border-top: 1px dashed var(--border-color); padding-top: 10px; }
  .history h3 { margin: 0 0 8px; font-size: 0.95rem; }
  .timeline-list { display: grid; gap: 8px; }
  .timeline-item { border: 1px solid var(--border-color); border-radius: 10px; padding: 10px; background: var(--bg-primary); display: grid; gap: 4px; }
  .timeline-head { display: flex; justify-content: space-between; gap: 10px; align-items: center; }
  .timeline-head strong { font-size: 0.9rem; }
  .timeline-head span, .timeline-meta { color: var(--text-secondary); font-size: 0.78rem; }
  .timeline-meta { display: flex; gap: 10px; }
  .timeline-item p { margin: 0; color: var(--text-secondary); font-size: 0.85rem; white-space: pre-wrap; word-break: break-word; }
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
