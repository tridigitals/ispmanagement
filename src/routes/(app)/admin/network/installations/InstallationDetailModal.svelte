<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';

  type TabId = 'info' | 'workflow' | 'onsite' | 'network' | 'timeline';

  let {
    tr,
    detailOpen = $bindable(),
    activeRow,
    closeDetail,
    statusClass,
    canReadAuditLogs,
    subscriptionStatusLabel,
    isGraceActive,
    subscriptionGraceDeadlineLabel,
    checklistTotal,
    checklistDoneCount,
    canManageWorkOrders,
    isAdminOwner,
    formAssignee = $bindable(),
    assigneeOptions,
    busyId,
    formSchedule = $bindable(),
    formNotes = $bindable(),
    canReleaseRow,
    canSaveAssignStep,
    saveAssignStep,
    assigneeLabel,
    resetToAssignStep,
    canSaveScheduleStep,
    saveScheduleStep,
    canStartActive,
    startFromDetail,
    claimWorkOrder,
    isUnassigned,
    isAssignedToCurrentUser,
    releaseWorkOrder,
    isClosedState,
    setStatus,
    canOperateRow,
    isPlanReady,
    checkCable = $bindable(),
    checkOnt = $bindable(),
    checkPppoe = $bindable(),
    checkSpeed = $bindable(),
    setOnsiteTaskChecked,
    savePlan,
    showCableMapDrawer = $bindable(),
    openCableDesigner,
    installationPhotos,
    uploadingPhotos,
    uploadInstallationPhotos,
    removeInstallationPhoto,
    getStorageContentUrl,
    installationSubscription,
    installationPppoeUsername = $bindable(),
    installationPppoePassword = $bindable(),
    installationPppoeComment = $bindable(),
    installationPppoeTarget = $bindable(),
    installationPppoeTargetOptions,
    installationPppoeAccount,
    savingInstallationPppoe,
    saveInstallationPppoe,
    applyInstallationPppoe,
    installationDhcpServerName = $bindable(),
    installationDhcpMacAddress = $bindable(),
    installationDhcpIpAddress = $bindable(),
    installationDhcpComment = $bindable(),
    installationDhcpQueueMode = $bindable(),
    installationDhcpQueueRateLimit = $bindable(),
    installationDhcpService,
    savingInstallationDhcp,
    saveInstallationDhcp,
    applyInstallationDhcp,
    installationPppoeMapping,
    installationManagedRadiusHint,
    installationManagedRadiusLoadError,
    installationManagedRadiusSetup,
    loadingInstallationPppoe,
    loadingInstallationDhcp,
    installationDhcpServerNameError,
    installationDhcpRouterError,
    installationDhcpMacAddressError,
    installationDhcpIpAddressError,
    installationDhcpQueueRateLimitError,
    installationDhcpQueueRateLimitPresets,
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
    installationQuickAssetDuplicates,
    installationQuickAssetCanSubmit,
    openInstallationQuickAsset,
    closeInstallationQuickAsset,
    createInstallationQuickAsset,
    updateInstallationQuickAssetField,
    continueToFinishStep,
    canCompleteActive,
    completeFromDetail,
    isAwaitingFirstPayment,
    canCreateMissingInvoice,
    creatingInvoiceId,
    createInvoiceFromDetail,
    rescheduleLoading,
    rescheduleRequest,
    formatDateTime,
    canReviewReschedule,
    rescheduleOverrideAt = $bindable(),
    rescheduleDecisionNotes = $bindable(),
    rescheduleDecisionBusy,
    approveRescheduleFromDetail,
    rejectRescheduleFromDetail,
    timelineLoading,
    timelineRows,
    handleCableMapSaved,
    cancelDialogOpen = $bindable(),
    cancelTarget,
    cancelReason = $bindable(),
    closeCancelDialog,
    confirmCancelFromDialog,
    hasValidCancelReason,
    openCancelDialog,
    effectiveStep,
  } = $props();

  let activeTab = $state<TabId>('info');

  const tabs = $derived.by(() => {
    const _tr = tr;
    return [
      { id: 'info' as TabId, label: _tr('common.info', 'Info'), icon: 'info' },
      { id: 'workflow' as TabId, label: _tr('admin.network.installations.workflow_tab', 'Workflow'), icon: 'clipboard-list' },
      { id: 'onsite' as TabId, label: _tr('admin.network.installations.onsite_tab', 'On-site'), icon: 'wrench' },
      { id: 'network' as TabId, label: _tr('admin.network.installations.network_tab', 'Network'), icon: 'wifi' },
      { id: 'timeline' as TabId, label: _tr('admin.network.installations.timeline_tab', 'Timeline'), icon: 'history' },
    ];
  });

  $effect(() => {
    if (!detailOpen || !activeRow) return;
    if (activeRow.status === 'pending') activeTab = 'workflow';
    else if (activeRow.status === 'in_progress') activeTab = 'onsite';
    else activeTab = 'info';
  });

  function backdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) closeDetail();
  }
  function backdropKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') closeDetail();
  }
  function cancelBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) closeCancelDialog();
  }
  function cancelBackdropKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') closeCancelDialog();
  }
</script>

{#if detailOpen && activeRow}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={backdropClick} onkeydown={backdropKeydown}>
    <div class="modal">
      <div class="modal-head">
        <div class="modal-head-left">
          <h2>{activeRow.customer_name || activeRow.customer_id}</h2>
          <span class={statusClass(activeRow.status)}>{activeRow.status}</span>
          <span class="wo-id">WO #{activeRow.id.slice(0, 8)}</span>
        </div>
        <button class="btn ghost icon-btn" onclick={closeDetail} aria-label={tr('common.close', 'Close')}>
          <Icon name="x" size={16} />
        </button>
      </div>

      <nav class="tab-bar">
        {#each tabs as tab}
          <button class="tab-btn" class:active={activeTab === tab.id} onclick={() => (activeTab = tab.id)}>
            <Icon name={tab.icon} size={14} />
            {tab.label}
          </button>
        {/each}
      </nav>

      <div class="tab-content">
        {#if activeTab === 'info'}
          <div class="meta-grid">
            <article class="meta-item"><span class="meta-label">{tr('common.customer', 'Customer')}</span><strong class="meta-value">{activeRow.customer_name || '—'}</strong></article>
            <article class="meta-item"><span class="meta-label">{tr('common.location', 'Location')}</span><strong class="meta-value">{activeRow.location || '—'}</strong></article>
            <article class="meta-item"><span class="meta-label">{tr('common.package', 'Package')}</span><strong class="meta-value">{activeRow.package_name || '—'}</strong></article>
            <article class="meta-item"><span class="meta-label">{tr('common.status', 'Status')}</span><span class="meta-value">{activeRow.status}</span></article>
            <article class="meta-item"><span class="meta-label">{tr('admin.network.installations.subscription_status', 'Subscription')}</span><span class="meta-value">{subscriptionStatusLabel}</span></article>
            <article class="meta-item"><span class="meta-label">{tr('common.assignee', 'Assignee')}</span><strong class="meta-value">{activeRow.assigned_to_name || '—'}</strong></article>
          </div>
          <div class="focus-panel">
            <div class="focus-copy">
              <span class="focus-kicker">{tr('admin.network.installations.current_step', 'Current')}</span>
              <strong>
                {activeRow.status === 'pending' ? tr('admin.network.installations.status_pending', 'Awaiting assignment')
                  : activeRow.status === 'in_progress' ? tr('admin.network.installations.status_active', 'Installation in progress')
                  : activeRow.status === 'completed' ? tr('admin.network.installations.status_done', 'Installation completed')
                  : tr('admin.network.installations.status_cancelled', 'Cancelled')}
              </strong>
            </div>
            {#if activeRow.status === 'completed' && isGraceActive}
              <div class="focus-chip"><span>{tr('admin.network.installations.grace_deadline', 'Grace until')}</span><strong>{subscriptionGraceDeadlineLabel}</strong></div>
            {:else if activeRow.status === 'in_progress'}
              <div class="focus-chip"><span>{tr('admin.network.installations.checklist', 'Checklist')}</span><strong>{checklistDoneCount}/{checklistTotal}</strong></div>
            {/if}
          </div>

        {:else if activeTab === 'workflow'}
          {#if canManageWorkOrders}
            <div class="wizard-card">
              {#if activeRow.status === 'pending'}
                <h3>{tr('admin.network.installations.step_assign', 'Assign & Schedule')}</h3>
                <p class="step-help">{tr('admin.network.installations.step_assign_help', 'Choose technician and schedule.')}</p>
                {#if isAdminOwner}
                  <label>{tr('common.assignee', 'Assignee')}
                    <select class="input" bind:value={formAssignee} disabled={busyId === activeRow.id}>
                      <option value="">{tr('admin.network.installations.assignee_placeholder', 'Select assignee')}</option>
                      {#each assigneeOptions as opt}<option value={opt.value}>{opt.label}</option>{/each}
                    </select>
                  </label>
                  <div class="assigned-summary"><span class="summary-label">{tr('common.assignee', 'Assignee')}</span><strong>{assigneeLabel(formAssignee)}</strong>
                    {#if isAdminOwner}<button class="btn ghost mini" type="button" onclick={resetToAssignStep}>{tr('common.change', 'Change')}</button>{/if}
                  </div>
                  <label>{tr('common.schedule', 'Schedule')}<input type="datetime-local" bind:value={formSchedule} disabled={busyId === activeRow.id} class="input" /></label>
                  <label class="notes">{tr('common.notes', 'Notes')}<textarea rows="3" bind:value={formNotes} class="input" placeholder={tr('admin.network.installations.notes_placeholder', '...')}></textarea></label>
                  <div class="modal-actions">
                    {#if canReleaseRow(activeRow)}<button class="btn ghost" onclick={() => activeRow && releaseWorkOrder(activeRow)} disabled={busyId === activeRow.id}>{tr('admin.network.installations.release', 'Release')}</button>{/if}
                    <button class="btn ghost" onclick={saveAssignStep} disabled={busyId === activeRow.id || !canSaveAssignStep}>{tr('admin.network.installations.save_assign', 'Save')}</button>
                    <button class="btn ghost" onclick={saveScheduleStep} disabled={busyId === activeRow.id || !canSaveScheduleStep}>{tr('admin.network.installations.save_schedule', 'Save Schedule')}</button>
                    <button class="btn" onclick={startFromDetail} disabled={busyId === activeRow.id || !canStartActive}>{tr('common.start', 'Start')}</button>
                  </div>
                {:else}
                  {#if isUnassigned(activeRow)}<button class="btn ghost" onclick={() => activeRow && claimWorkOrder(activeRow)} disabled={busyId === activeRow.id}>{tr('admin.network.installations.claim_work_order', 'Claim')}</button>
                  {:else if isAssignedToCurrentUser(activeRow)}<p class="helper-text">{tr('admin.network.installations.already_taken_by_you', 'You took this WO.')}</p>
                  {:else}<p class="helper-text">{tr('admin.network.installations.taken_by_other', 'Taken by another technician.')}</p>{/if}
                {/if}
              {:else if isClosedState}
                <h3>{activeRow.status === 'completed' ? tr('common.complete', 'Completed') : tr('common.cancelled', 'Cancelled')}</h3>
                <p class="step-help">{activeRow.status === 'completed' ? (isAwaitingFirstPayment ? tr('admin.network.installations.final_waiting_payment', 'Waiting first payment.') : tr('admin.network.installations.final_completed', 'Completed and active.')) : tr('admin.network.installations.final_cancelled', 'Cancelled.')}</p>
                {#if selectedTerminalAssetLabel}<div class="activation-ready"><div>{tr('admin.network.installations.terminal_asset_selected', 'Terminal')}: <strong>{selectedTerminalAssetLabel}</strong></div></div>{/if}
                {#if canCreateMissingInvoice}<button class="btn ghost" type="button" onclick={createInvoiceFromDetail} disabled={creatingInvoiceId === activeRow.id}><Icon name="file-plus" size={14} /> {tr('admin.network.installations.create_invoice', 'Create invoice')}</button>{/if}
                {#if activeRow.status === 'cancelled'}<label class="notes"><textarea rows="2" bind:value={formNotes} class="input" placeholder="Reopen reason"></textarea></label><button class="btn ghost" onclick={() => activeRow && setStatus(activeRow, 'reopen', formNotes)} disabled={busyId === activeRow.id}>{tr('common.reopen', 'Reopen')}</button>{/if}
              {/if}
            </div>
          {/if}

        {:else if activeTab === 'onsite'}
          {#if activeRow.status === 'in_progress'}
            <div class="wizard-card">
              <h3>{tr('admin.network.installations.step_onsite', 'On-site Installation')}</h3>
              <fieldset class="checklist">
                <legend>{tr('admin.network.installations.checklist', 'Checklist')} <span class="progress-inline">{checklistDoneCount}/{checklistTotal}</span></legend>
                {#each [{ key: 'cable', label: 'Cable Installation', desc: 'Physical cable routing and termination', checked: checkCable },{ key: 'ont', label: 'ONT/ONU Setup', desc: 'Device mounted, powered, and connected', checked: checkOnt },{ key: 'pppoe', label: 'PPPoE/DHCP Test', desc: 'Internet connectivity verified', checked: checkPppoe },{ key: 'speed', label: 'Speed Test', desc: 'Bandwidth matches subscription', checked: checkSpeed }] as item, i}
                  <label class="check-item" class:is-done={item.checked}>
                    <input type="checkbox" checked={item.checked} onchange={(e) => setOnsiteTaskChecked(i, (e.currentTarget as HTMLInputElement).checked)} />
                    <span class="check-indicator">{#if item.checked}<Icon name="check" size={14} />{/if}</span>
                    <span class="check-content"><strong>{item.label}</strong><small>{item.desc}</small></span>
                  </label>
                {/each}
              </fieldset>
              <div class="cable-designer-card"><div><strong>{tr('admin.network.installations.cable_route_title', 'Cable Route')}</strong><p>{tr('admin.network.installations.cable_route_desc', 'Draw in Topology Map.')}</p></div><button class="btn ghost" type="button" onclick={openCableDesigner}><Icon name="map-pin" size={14} /> {tr('admin.network.installations.open_cable_designer', 'Draw')}</button></div>
              <section class="photos-card">
                <div class="photos-head"><strong>{tr('admin.network.installations.photos_title', 'Photos')}</strong><label class="btn ghost upload-btn"><Icon name="image" size={14} /> {uploadingPhotos ? tr('common.loading', '...') : tr('admin.network.installations.photos_add', 'Add')}<input type="file" accept="image/*" multiple onchange={uploadInstallationPhotos} disabled={uploadingPhotos} class="hidden-input" /></label></div>
                {#if installationPhotos.length > 0}<div class="photo-grid">{#each installationPhotos as file}<article class="photo-item"><img src={getStorageContentUrl(file.id)} alt={file.original_name || 'Photo'} loading="lazy" /><div class="photo-meta"><button class="btn danger mini" type="button" onclick={() => removeInstallationPhoto(file.id)}><Icon name="trash" size={12} /></button></div></article>{/each}</div>{:else}<p class="helper-text">{tr('admin.network.installations.photos_empty', 'No photos yet.')}</p>{/if}
              </section>
              <label class="notes">{tr('common.notes', 'Notes')}<textarea rows="3" bind:value={formNotes} class="input" placeholder={tr('admin.network.installations.notes_on_site_placeholder', '...')}></textarea></label>
              <div class="modal-actions"><button class="btn ghost" onclick={savePlan} disabled={busyId === activeRow.id}>{tr('admin.network.installations.save_plan', 'Save Plan')}</button></div>
            </div>
          {/if}

        {:else if activeTab === 'network'}
          <div class="wizard-card">
            <h3>{tr('admin.network.installations.network_config', 'Network Configuration')}</h3>
            {#if activeRow?.package_provisioning_type === 'dhcp_static'}
              <h4>DHCP Static</h4>
              <div class="form-grid two-col compact">
                <label><span class="summary-label">{tr('admin.network.installations.dhcp_server', 'DHCP Server')}</span><input class="input" class:error={!!installationDhcpServerNameError} bind:value={installationDhcpServerName} /></label>
                <label><span class="summary-label">{tr('admin.network.installations.mac_address', 'MAC Address')}</span><input class="input" class:error={!!installationDhcpMacAddressError} bind:value={installationDhcpMacAddress} /></label>
                <label><span class="summary-label">{tr('admin.network.installations.ip_address', 'IP Address')}</span><input class="input" class:error={!!installationDhcpIpAddressError} bind:value={installationDhcpIpAddress} /></label>
                <label><span class="summary-label">{tr('admin.network.installations.queue_mode', 'Queue')}</span><select class="input" bind:value={installationDhcpQueueMode}><option value="none">No queue</option><option value="simple_queue">Simple queue</option></select></label>
              </div>
              {#if installationDhcpQueueMode === 'simple_queue'}<label><span class="summary-label">{tr('admin.network.installations.queue_rate_limit', 'Rate Limit')}</span><input class="input" bind:value={installationDhcpQueueRateLimit} placeholder="20M/20M" /></label>{/if}
              <label><span class="summary-label">{tr('common.comment', 'Comment')}</span><input class="input" bind:value={installationDhcpComment} /></label>
              <div class="modal-actions"><button class="btn ghost" onclick={saveInstallationDhcp} disabled={savingInstallationDhcp}>{savingInstallationDhcp ? '...' : installationDhcpService ? tr('admin.network.installations.save_reapply', 'Save & Reapply') : tr('admin.network.installations.save_activate', 'Save & Activate')}</button>{#if installationDhcpService}<button class="btn ghost" onclick={applyInstallationDhcp} disabled={savingInstallationDhcp}>{tr('admin.network.installations.reapply', 'Reapply')}</button>{/if}</div>
            {:else}
              <h4>PPPoE</h4>
              <div class="form-grid two-col compact">
                <label><span class="summary-label">{tr('admin.network.installations.pppoe_username', 'Username')}</span><input class="input" bind:value={installationPppoeUsername} placeholder="pppoe username" /></label>
                <label><span class="summary-label">{tr('admin.network.installations.pppoe_password', 'Password')}</span><input class="input" type="password" bind:value={installationPppoePassword} placeholder="pppoe password" /></label>
              </div>
              {#if installationPppoeTargetOptions.length > 1}<label><span class="summary-label">{tr('admin.network.installations.provision_to', 'Provision to')}</span><select class="input" bind:value={installationPppoeTarget}>{#each installationPppoeTargetOptions as option (option.value)}<option value={option.value} disabled={option.disabled}>{option.label}</option>{/each}</select></label>{/if}
              <label><span class="summary-label">{tr('common.comment', 'Comment')}</span><input class="input" bind:value={installationPppoeComment} /></label>
              {#if installationPppoeAccount}<div class="pppoe-existing"><span>{tr('admin.network.installations.pppoe_existing', 'Existing:')}</span><strong>{installationPppoeAccount.username}</strong></div>{/if}
              <div class="modal-actions"><button class="btn ghost" onclick={saveInstallationPppoe} disabled={savingInstallationPppoe}>{savingInstallationPppoe ? '...' : installationPppoeAccount ? tr('admin.network.installations.save_reapply', 'Save & Reapply') : tr('admin.network.installations.save_activate', 'Save & Activate')}</button>{#if installationPppoeAccount}<button class="btn ghost" onclick={applyInstallationPppoe} disabled={savingInstallationPppoe}>{tr('admin.network.installations.reapply', 'Reapply')}</button>{/if}</div>
            {/if}

            <h4 class="mt">{tr('admin.network.installations.step_assets', 'Asset Binding')}</h4>
            {#if loadingInstallationAssets}<p class="helper-text">Loading...</p>
            {:else}
              <label><span class="summary-label">{tr('admin.network.installations.terminal_asset', 'Terminal (ONT/ONU)')}</span><select class="input" bind:value={installationTerminalAssetId} onchange={handleInstallationTerminalAssetChange}><option value="">{tr('admin.network.installations.select_terminal_asset', 'Select')}</option>{#each installationTerminalAssetOptions as option}<option value={option.value}>{option.label}</option>{/each}</select></label>
              <label><span class="summary-label">{tr('admin.network.installations.parent_asset_optional', 'Parent (Optional)')}</span><select class="input" bind:value={installationParentAssetId} onchange={handleInstallationParentAssetChange}><option value="">{tr('admin.network.installations.no_parent_asset', 'None')}</option>{#each installationParentAssetOptions as option}<option value={option.value}>{option.label}</option>{/each}</select></label>
              <div class="modal-actions"><button class="btn ghost" type="button" onclick={openInstallationQuickAsset}><Icon name="plus" size={14} /> {tr('admin.network.installations.quick_create_terminal_asset', 'Create New')}</button></div>
              {#if installationQuickAssetOpen}
                <section class="quick-asset-card">
                  <div class="quick-asset-head"><strong>{tr('admin.network.installations.quick_create_title', 'Quick Create')}</strong><button class="btn ghost mini" type="button" onclick={closeInstallationQuickAsset}>{tr('common.close', 'Close')}</button></div>
                  <div class="form-grid two-col compact">
                    <label class="summary-field">{tr('admin.network.installations.quick_asset_type', 'Type')}<select class="input" value={installationQuickAssetDraft.asset_type} onchange={(event) => updateInstallationQuickAssetField('asset_type', (event.currentTarget as HTMLSelectElement).value)}><option value="ont">ONT</option><option value="onu">ONU</option></select></label>
                    <label class="summary-field">{tr('admin.network.installations.quick_asset_name', 'Name')}<input class="input" value={installationQuickAssetDraft.name} oninput={(event) => updateInstallationQuickAssetField('name', (event.currentTarget as HTMLInputElement).value)} /></label>
                    <label class="summary-field">{tr('admin.network.installations.quick_asset_serial', 'Serial')}<input class="input" value={installationQuickAssetDraft.serial_number} oninput={(event) => updateInstallationQuickAssetField('serial_number', (event.currentTarget as HTMLInputElement).value)} /></label>
                    <label class="summary-field">{tr('admin.network.installations.quick_asset_code', 'Code')}<input class="input" value={installationQuickAssetDraft.code} oninput={(event) => updateInstallationQuickAssetField('code', (event.currentTarget as HTMLInputElement).value)} /></label>
                  </div>
                  <div class="modal-actions"><button class="btn ghost" type="button" onclick={closeInstallationQuickAsset}>{tr('common.cancel', 'Cancel')}</button><button class="btn" type="button" onclick={createInstallationQuickAsset} disabled={!installationQuickAssetCanSubmit}>{creatingInstallationQuickAsset ? '...' : tr('admin.network.installations.quick_create_submit', 'Create')}</button></div>
                </section>
              {/if}
            {/if}

            {#if activeRow.status === 'in_progress'}<div class="modal-actions"><button class="btn success" onclick={completeFromDetail} disabled={busyId === activeRow.id || !canCompleteActive}>{tr('common.complete', 'Complete')}</button></div>{/if}
          </div>

        {:else if activeTab === 'timeline'}
          {#if rescheduleRequest}
            <div class="wizard-card">
              <h3>{tr('admin.network.installations.reschedule_pending_title', 'Reschedule Request')}</h3>
              <div class="reschedule-request-card">
                <div class="reschedule-request-head"><strong>Pending</strong><span>{formatDateTime(rescheduleRequest.created_at)}</span></div>
                <div><strong>{tr('common.requested_by', 'By')}:</strong> {rescheduleRequest.requested_by_name || '-'}</div>
                <div><strong>{tr('common.schedule', 'New')}:</strong> {formatDateTime(rescheduleRequest.requested_schedule)}</div>
                {#if rescheduleRequest.reason}<p>{rescheduleRequest.reason}</p>{/if}
                {#if canReviewReschedule}
                  <label><span class="summary-label">{tr('admin.network.installations.override_schedule_optional', 'Override (optional)')}</span><input type="datetime-local" class="input" bind:value={rescheduleOverrideAt} disabled={rescheduleDecisionBusy} /></label>
                  <label><span class="summary-label">{tr('common.notes', 'Notes')}</span><textarea rows="2" class="input" bind:value={rescheduleDecisionNotes} disabled={rescheduleDecisionBusy}></textarea></label>
                  <div class="modal-actions"><button class="btn ghost" onclick={approveRescheduleFromDetail} disabled={rescheduleDecisionBusy}>{tr('common.approve', 'Approve')}</button><button class="btn danger" onclick={rejectRescheduleFromDetail} disabled={rescheduleDecisionBusy}>{tr('common.reject', 'Reject')}</button></div>
                {/if}
              </div>
            </div>
          {/if}
          {#if canReadAuditLogs}
            <div class="wizard-card">
              <h3>{tr('admin.network.installations.timeline', 'Audit Timeline')}</h3>
              {#if timelineLoading}<p class="helper-text">Loading...</p>
              {:else if timelineRows.length === 0}<p class="helper-text">{tr('common.no_data', 'No data')}</p>
              {:else}<div class="timeline-list">{#each timelineRows as log}<article class="timeline-item"><div class="timeline-head"><strong>{log.action}</strong><span>{formatDateTime(log.created_at)}</span></div><div class="timeline-meta"><span>{log.user_name || log.user_email || '-'}</span>{#if log.ip_address}<span>{log.ip_address}</span>{/if}</div>{#if log.details}<p>{log.details}</p>{/if}</article>{/each}</div>{/if}
            </div>
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if cancelDialogOpen && cancelTarget}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={cancelBackdropClick} onkeydown={cancelBackdropKeydown}>
    <div class="modal cancel-modal">
      <div class="modal-head"><h2>{tr('common.cancel', 'Cancel')} WO</h2><button class="btn ghost icon-btn" onclick={closeCancelDialog} aria-label={tr('common.close', 'Close')}><Icon name="x" size={16} /></button></div>
      <p class="step-help">{tr('admin.network.installations.cancel_reason_required', 'Min 10 chars reason.')}</p>
      <label class="notes"><textarea rows="4" bind:value={cancelReason} class="input"></textarea></label>
      <div class="modal-actions"><button class="btn ghost" onclick={closeCancelDialog} disabled={busyId === cancelTarget.id}>{tr('common.close', 'Close')}</button><button class="btn danger" onclick={confirmCancelFromDialog} disabled={busyId === cancelTarget.id || !hasValidCancelReason}>{tr('common.confirm', 'Confirm Cancel')}</button></div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop { position: fixed; inset: 0; background: color-mix(in srgb, var(--bg-app) 72%, transparent); display: grid; place-items: center; padding: 20px; z-index: 1000; }
  .modal { width: min(720px, 100%); max-height: calc(100vh - 40px); overflow: hidden; display: flex; flex-direction: column; border-radius: 14px; background: var(--bg-surface); border: 1px solid var(--border-color); box-shadow: var(--shadow-md); }
  .cancel-modal { width: min(480px, 100%); overflow: auto; display: grid; gap: 14px; padding: 16px; }
  .modal-head { display: flex; justify-content: space-between; align-items: center; padding: 14px 16px; border-bottom: 1px solid var(--border-color); flex-shrink: 0; }
  .modal-head-left { display: flex; align-items: center; gap: 10px; }
  .modal-head h2 { margin: 0; font-size: 1.1rem; }
  .wo-id { color: var(--text-secondary); font-size: 0.8rem; }
  .tab-bar { display: flex; gap: 0; border-bottom: 1px solid var(--border-color); padding: 0 8px; flex-shrink: 0; overflow-x: auto; }
  .tab-btn { padding: 10px 16px; border: none; background: transparent; color: var(--text-secondary); font-size: 0.85rem; cursor: pointer; display: flex; align-items: center; gap: 6px; border-bottom: 2px solid transparent; white-space: nowrap; transition: color 140ms, border-color 140ms; font-family: inherit; }
  .tab-btn.active { color: var(--color-primary); border-bottom-color: var(--color-primary); }
  .tab-btn:hover { color: var(--text-primary); }
  .tab-content { flex: 1; overflow-y: auto; padding: 14px 16px; }
  .meta-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px 14px; }
  .meta-item { border: 1px solid var(--border-color); border-radius: 10px; background: var(--bg-surface); padding: 10px 12px; display: grid; gap: 3px; }
  .meta-label { color: var(--text-secondary); font-size: 0.75rem; letter-spacing: 0.04em; text-transform: uppercase; font-weight: 600; }
  .meta-value { color: var(--text-primary); font-size: 0.96rem; font-weight: 800; }
  .focus-panel { border: 1px solid color-mix(in srgb, var(--color-primary) 32%, var(--border-color)); border-radius: 14px; padding: 14px; display: flex; justify-content: space-between; gap: 10px; margin-top: 12px; }
  .focus-copy { display: grid; gap: 4px; min-width: 0; }
  .focus-kicker { color: var(--color-primary); font-size: 0.72rem; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; }
  .focus-copy strong { color: var(--text-primary); font-size: 1rem; }
  .focus-chip { min-width: 150px; border: 1px solid var(--border-color); border-radius: 12px; background: var(--bg-primary); padding: 12px 14px; display: grid; gap: 4px; }
  .focus-chip span { color: var(--color-primary); font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.06em; font-weight: 700; }
  .focus-chip strong { color: var(--text-primary); font-size: 0.95rem; }
  .wizard-card { border: 1px solid var(--border-color); border-radius: 12px; background: var(--bg-surface); padding: 16px 16px 12px; display: grid; gap: 10px; margin-bottom: 14px; }
  .wizard-card h3 { margin: 0; font-size: 0.95rem; }
  .step-help { margin: 0; font-size: 0.9rem; color: var(--text-secondary); }
  .assigned-summary { border: 1px solid var(--border-color); border-radius: 10px; padding: 10px 12px; display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .summary-label { color: var(--text-secondary); font-size: 0.8rem; letter-spacing: 0.03em; text-transform: uppercase; font-weight: 600; display: block; margin-bottom: 4px; }
  .form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
  label { display: grid; gap: 6px; font-size: 0.92rem; }
  .input { background: var(--bg-primary); color: var(--text-primary); border: 1px solid var(--border-color); border-radius: 10px; padding: 8px 10px; font-family: inherit; font-size: 0.92rem; width: 100%; box-sizing: border-box; }
  .input.error { border-color: var(--color-danger-500, #ef4444); }
  .checklist { border: 1px solid var(--border-color); border-radius: 10px; padding: 12px; display: grid; gap: 8px; }
  .progress-inline { margin-left: 8px; font-size: 0.78rem; color: var(--color-primary); font-weight: 700; }
  .check-item { border: 1px solid var(--border-color); background: var(--bg-surface); border-radius: 10px; padding: 10px 12px; display: flex; align-items: center; gap: 10px; cursor: pointer; transition: border-color 120ms; }
  .check-item:hover { border-color: var(--color-primary); }
  .check-item input[type='checkbox'] { position: absolute; opacity: 0; width: 1px; height: 1px; pointer-events: none; }
  .check-indicator { width: 22px; height: 22px; border-radius: 7px; border: 1px solid var(--border-color); background: var(--bg-primary); display: grid; place-items: center; flex-shrink: 0; }
  .check-content { display: grid; gap: 3px; color: var(--text-primary); }
  .check-content strong { font-size: 0.96rem; }
  .check-content small { color: var(--text-secondary); font-size: 0.8rem; }
  .check-item.is-done { border-color: var(--color-success); background: var(--bg-success); }
  .check-item.is-done .check-indicator { border-color: var(--color-success); background: var(--color-success); color: var(--bg-app); }
  .cable-designer-card { border: 1px solid var(--border-color); border-radius: 12px; padding: 12px; display: flex; justify-content: space-between; align-items: center; gap: 10px; }
  .cable-designer-card p { margin: 4px 0 0; font-size: 0.85rem; color: var(--text-secondary); }
  .photos-card { border: 1px solid var(--border-color); border-radius: 10px; padding: 10px; display: grid; gap: 10px; background: var(--bg-primary); }
  .photos-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .upload-btn { position: relative; overflow: hidden; }
  .upload-btn .hidden-input { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
  .photo-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(130px, 1fr)); gap: 8px; }
  .photo-item { border: 1px solid var(--border-color); border-radius: 10px; background: var(--bg-surface); overflow: hidden; }
  .photo-item img { width: 100%; height: 92px; object-fit: cover; border-radius: 6px; border: 1px solid var(--border-color); }
  .photo-meta { display: flex; align-items: center; justify-content: flex-end; gap: 6px; padding: 6px 8px; }
  .pppoe-existing { border: 1px solid color-mix(in srgb, var(--color-success) 28%, var(--border-color)); border-radius: 10px; padding: 8px 12px; display: flex; gap: 8px; align-items: center; background: var(--bg-success); }
  .pppoe-existing span:first-child { color: var(--text-success); font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.06em; font-weight: 700; }
  .quick-asset-card { border: 1px solid var(--border-color); border-radius: 12px; background: var(--bg-surface); padding: 12px; display: grid; gap: 10px; }
  .quick-asset-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .activation-ready { border: 1px dashed var(--border-color); border-radius: 12px; padding: 12px; display: grid; gap: 6px; }
  .reschedule-request-card { border: 1px solid color-mix(in srgb, var(--color-warning) 42%, var(--border-color)); border-radius: 12px; padding: 12px; display: grid; gap: 6px; background: var(--bg-warning); font-size: 0.9rem; }
  .reschedule-request-head { display: flex; justify-content: space-between; align-items: center; gap: 10px; font-size: 0.85rem; }
  .reschedule-request-card p { margin: 0; font-size: 0.86rem; color: var(--text-primary); white-space: pre-wrap; word-break: break-word; font-weight: 600; }
  .timeline-list { display: grid; gap: 8px; }
  .timeline-item { border: 1px solid var(--border-color); border-radius: 10px; padding: 10px; background: var(--bg-primary); }
  .timeline-head { display: flex; justify-content: space-between; gap: 10px; align-items: center; }
  .timeline-head strong { font-size: 0.9rem; }
  .timeline-head span, .timeline-meta { color: var(--text-secondary); font-size: 0.78rem; }
  .timeline-meta { display: flex; gap: 10px; }
  .timeline-item p { margin: 0; color: var(--text-secondary); font-size: 0.85rem; white-space: pre-wrap; word-break: break-word; }
  .notes textarea { resize: vertical; min-height: 80px; }
  .helper-text { margin: 0; color: var(--text-secondary); font-size: 0.85rem; }
  .btn { border: 1px solid var(--border-color); border-radius: 12px; background: var(--color-primary); color: var(--bg-app); font-weight: 800; padding: 8px 12px; cursor: pointer; display: inline-flex; align-items: center; gap: 6px; font-family: inherit; font-size: 0.9rem; }
  .btn.mini { padding: 5px 9px; font-size: 0.76rem; border-radius: 10px; }
  .btn.ghost { background: transparent; color: var(--text-primary); }
  .btn.success { border-color: color-mix(in srgb, var(--color-success) 38%, var(--border-color)); background: var(--bg-success); color: var(--text-success); }
  .btn.danger { border-color: color-mix(in srgb, var(--color-danger) 34%, var(--border-color)); background: color-mix(in srgb, var(--color-danger) 12%, var(--bg-primary)); color: var(--color-danger); }
  .btn:disabled { opacity: 0.55; cursor: not-allowed; }
  .icon-btn { width: 36px; height: 36px; padding: 0; justify-content: center; }
  .modal-actions { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; margin-top: 4px; }
  .mt { margin-top: 14px; }
  .status { display: inline-flex; border-radius: 999px; border: 1px solid var(--border-color); padding: 2px 10px; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; }
  .status.pending { border-color: color-mix(in srgb, var(--color-warning) 40%, var(--border-color)); color: var(--color-warning); }
  .status.progress { border-color: color-mix(in srgb, var(--color-primary) 40%, var(--border-color)); color: var(--color-primary); }
  .status.completed { border-color: color-mix(in srgb, var(--color-success) 40%, var(--border-color)); color: var(--text-success); }
  .status.cancelled { border-color: color-mix(in srgb, var(--color-danger) 40%, var(--border-color)); color: var(--color-danger); }
  @media (max-width: 640px) { .meta-grid, .form-grid { grid-template-columns: 1fr; } .focus-panel { flex-direction: column; } }
</style>
