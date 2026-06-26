1|<script lang="ts">
2|  import { t } from 'svelte-i18n';
3|  import Icon from '$lib/components/ui/Icon.svelte';
4|  import IncidentRunbook from '$lib/components/network/IncidentRunbook.svelte';
5|  import IncidentTimeline from '$lib/components/network/IncidentTimeline.svelte';
6|
7|  type IncidentRow = {
8|    id: string;
9|    router_id: string;
10|    interface_name?: string | null;
11|    incident_type: string;
12|    severity: string;
13|    status: string;
14|    title: string;
15|    message: string;
16|    owner_user_id?: string | null;
17|    notes?: string | null;
18|    is_auto_escalated?: boolean;
19|    escalated_at?: string | null;
20|    first_seen_at?: string | null;
21|    acked_at?: string | null;
22|    last_seen_at: string;
23|    resolved_at?: string | null;
24|    updated_at: string;
25|  };
26|
27|  type RouterRow = {
28|    identity?: string | null;
29|    name?: string | null;
30|    host?: string | null;
31|    port?: number | null;
32|    latency_ms?: number | null;
33|  };
34|
35|  type RouterMetricRow = {
36|    cpu_load?: number | null;
37|    rx_bps?: number | null;
38|    tx_bps?: number | null;
39|    free_memory_bytes?: number | null;
40|    total_memory_bytes?: number | null;
41|  };
42|
43|  type TeamMemberLite = {
44|    user_id?: string;
45|    name: string;
46|    email: string;
47|  };
48|
49|  type RunbookStep = {
50|    title: string;
51|    detail?: string;
52|    command?: string;
53|  };
54|
55|  type ActivityItem = {
56|    ts: string;
57|    title: string;
58|    detail?: string;
59|  };
60|
61|  type ImpactCustomer = {
62|    assignment_id: string;
63|    assignment_status: string;
64|    subscription_status?: string | null;
65|    customer_name: string;
66|    location_label?: string | null;
67|    selected_node_name?: string | null;
68|    impacted_via_node?: boolean;
69|    impacted_via_link?: boolean;
70|  };
71|
72|  let {
73|    open = false,
74|    incident = null,
75|    loading = false,
76|    router = null,
77|    metric = null,
78|    teamMembers = [],
79|    selectedOwnerId = '',
80|    draftNotes = '',
81|    saving = false,
82|    canManage = false,
83|    emailNotifyEnabled = false,
84|    slaState = 'ok',
85|    slaOpenDuration = '—',
86|    appTimezone = 'UTC',
87|    runbookSteps = [],
88|    activityItems = [],
89|    impactedLoading = false,
90|    impactedCustomers = [],
91|    ownerLabel,
92|    typeLabel,
93|    severityLabel,
94|    formatDateTime,
95|    formatBps,
96|    memoryUsePct,
97|    onClose,
98|    onOpenRouter,
99|    onAcknowledge,
100|    onResolve,
101|    onSave,
102|    onOpenNetworkSettings,
103|    onOwnerChange,
104|    onNotesChange,
105|    onCopyRunbookCommand,
106|    onAddRunbookStep,
107|  }: {
108|    open?: boolean;
109|    incident?: IncidentRow | null;
110|    loading?: boolean;
111|    router?: RouterRow | null;
112|    metric?: RouterMetricRow | null;
113|    teamMembers?: TeamMemberLite[];
114|    selectedOwnerId?: string;
115|    draftNotes?: string;
116|    saving?: boolean;
117|    canManage?: boolean;
118|    emailNotifyEnabled?: boolean;
119|    slaState?: 'ok' | 'warn' | 'breach' | string;
120|    slaOpenDuration?: string;
121|    appTimezone?: string;
122|    runbookSteps?: RunbookStep[];
123|    activityItems?: ActivityItem[];
124|    impactedLoading?: boolean;
125|    impactedCustomers?: ImpactCustomer[];
126|    ownerLabel: (ownerUserId?: string | null) => string;
127|    typeLabel: (incidentType: string) => string;
128|    severityLabel: (severity: string) => string;
129|    formatDateTime: (value: string, options?: Record<string, unknown>) => string;
130|    formatBps: (value?: number | null) => string;
131|    memoryUsePct: (total?: number | null, free?: number | null) => number | null;
132|    onClose: () => void;
133|    onOpenRouter: (routerId: string) => void;
134|    onAcknowledge: (id: string) => void | Promise<void>;
135|    onResolve: (id: string) => void | Promise<void>;
136|    onSave: () => void | Promise<void>;
137|    onOpenNetworkSettings: () => void;
138|    onOwnerChange: (value: string) => void;
139|    onNotesChange: (value: string) => void;
140|    onCopyRunbookCommand: (command: string) => void | Promise<void>;
141|    onAddRunbookStep: (step: RunbookStep) => void;
142|  } = $props();
143|</script>
144|
145|{#if open && incident}
146|  <button class="drawer-backdrop" type="button" onclick={onClose} aria-label={$t('common.close')}></button>
147|  <aside class="drawer" aria-label={$t('common.details')}>
148|    <div class="drawer-head">
149|      <div>
150|        <div class="drawer-title">{$t('common.details')}</div>
151|        <div class="drawer-sub">{incident.title}</div>
152|      </div>
153|      <button class="icon-btn" type="button" onclick={onClose} title={$t('common.close')}>
154|        <Icon name="x" size={16} />
155|      </button>
156|    </div>
157|
158|    <div class="drawer-body">
159|      {#if loading}
160|        <div class="muted">{$t('common.loading')}</div>
161|      {/if}
162|
163|      <div class="detail-grid">
164|        <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.status')}</span><span class="mono">{incident.status}</span></div>
165|        <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.type')}</span><span class="mono">{typeLabel(incident.incident_type)}</span></div>
166|        <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.severity')}</span><span class="mono">{severityLabel(incident.severity)}</span></div>
167|        <div class="drow">
168|          <span class="muted">{$t('admin.network.incidents.labels.auto_escalated')}</span>
169|          <span class="mono">
170|            {incident.is_auto_escalated
171|              ? formatDateTime(incident.escalated_at || incident.updated_at, { timeZone: appTimezone })
172|              : ($t('common.no') || 'No')}
173|          </span>
174|        </div>
175|        <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.seen')}</span><span class="mono">{formatDateTime(incident.last_seen_at, { timeZone: appTimezone })}</span></div>
176|        <div class="drow">
177|          <span class="muted">{$t('admin.network.incidents.drawer.email_notify')}</span>
178|          <span class="mono">
179|            <span class:flag-on={emailNotifyEnabled} class:flag-off={!emailNotifyEnabled} class="flag">
180|              {emailNotifyEnabled
181|                ? $t('admin.network.incidents.drawer.on') || 'On'
182|                : $t('admin.network.incidents.drawer.off') || 'Off'}
183|            </span>
184|          </span>
185|        </div>
186|        <div class="drow">
187|          <span class="muted">{$t('admin.network.incidents.sla.title')}</span>
188|          <span class="mono">
189|            <span class="sla-badge" class:warn={slaState === 'warn'} class:breach={slaState === 'breach'}>
190|              {slaOpenDuration}
191|            </span>
192|          </span>
193|        </div>
194|        <div class="drow"><span class="muted">{$t('admin.network.incidents.drawer.assignee')}</span><span class="mono">{ownerLabel(incident.owner_user_id)}</span></div>
195|        <div class="drow"><span class="muted">{$t('admin.network.incidents.labels.router')}</span><span class="mono">{router?.identity || router?.name || incident.router_id}</span></div>
196|        <div class="drow"><span class="muted">{$t('admin.network.incidents.labels.interface')}</span><span class="mono">{incident.interface_name || '-'}</span></div>
197|        {#if router}
198|          <div class="drow"><span class="muted">{$t('admin.network.incidents.labels.host')}</span><span class="mono">{router.host}:{router.port}</span></div>
199|          <div class="drow"><span class="muted">{$t('admin.network.incidents.labels.latency')}</span><span class="mono">{router.latency_ms == null ? '—' : `${router.latency_ms} ms`}</span></div>
200|        {/if}
201|        {#if metric}
202|          <div class="drow"><span class="muted">{$t('admin.network.incidents.labels.cpu')}</span><span class="mono">{metric.cpu_load == null ? '—' : `${metric.cpu_load}%`}</span></div>
203|          <div class="drow"><span class="muted">{$t('admin.network.incidents.labels.rx_tx')}</span><span class="mono">{formatBps(metric.rx_bps)} / {formatBps(metric.tx_bps)}</span></div>
204|          <div class="drow"><span class="muted">{$t('admin.network.incidents.labels.memory_use')}</span><span class="mono">{memoryUsePct(metric.total_memory_bytes, metric.free_memory_bytes) == null ? '—' : `${memoryUsePct(metric.total_memory_bytes, metric.free_memory_bytes)}%`}</span></div>
205|        {/if}
206|      </div>
207|
208|      <div class="detail-message">{incident.message}</div>
209|
210|      <div class="impact-card">
211|        <div class="impact-head">
212|          <div class="impact-title">
213|            {$t('admin.network.incidents.impact.title')}
214|          </div>
215|          <div class="impact-count">{impactedCustomers.length}</div>
216|        </div>
217|        {#if impactedLoading}
218|          <div class="muted">{$t('common.loading')}</div>
219|        {:else if impactedCustomers.length === 0}
220|          <div class="muted">
221|            {$t('admin.network.incidents.impact.empty')}
222|          </div>
223|        {:else}
224|          <div class="impact-list">
225|            {#each impactedCustomers.slice(0, 10) as item}
226|              <div class="impact-item">
227|                <div>
228|                  <div class="impact-name">{item.customer_name}</div>
229|                  <div class="impact-meta">{item.location_label || ($t('common.na') || 'N/A')}</div>
230|                </div>
231|                <div class="impact-tags">
232|                  {#if item.impacted_via_node}
233|                    <span class="impact-tag">{$t('admin.network.incidents.labels.via_node')}</span>
234|                  {/if}
235|                  {#if item.impacted_via_link}
236|                    <span class="impact-tag">{$t('admin.network.incidents.labels.via_link')}</span>
237|                  {/if}
238|                </div>
239|              </div>
240|            {/each}
241|          </div>
242|        {/if}
243|      </div>
244|
245|      <div class="detail-edit">
246|        <div class="field">
247|          <label for="incident-owner">{$t('admin.network.incidents.drawer.assignee')}</label>
248|          {#if canManage}
249|            <select
250|              id="incident-owner"
251|              class="input"
252|              value={selectedOwnerId}
253|              onchange={(e) => onOwnerChange((e.currentTarget as HTMLSelectElement).value)}
254|            >
255|              <option value="">{($t('admin.network.incidents.drawer.unassigned') || 'Unassigned')}</option>
256|              {#each teamMembers as member}
257|                <option value={member.user_id}>{member.name} ({member.email})</option>
258|              {/each}
259|            </select>
260|          {:else}
261|            <div class="readonly">{ownerLabel(incident.owner_user_id)}</div>
262|          {/if}
263|        </div>
264|        <div class="field">
265|          <label for="incident-notes">{$t('admin.network.incidents.drawer.notes')}</label>
266|          {#if canManage}
267|            <textarea
268|              id="incident-notes"
269|              class="textarea"
270|              rows="4"
271|              value={draftNotes}
272|              oninput={(e) => onNotesChange((e.currentTarget as HTMLTextAreaElement).value)}
273|              placeholder={$t('admin.network.incidents.drawer.notes_placeholder')}
274|            ></textarea>
275|          {:else}
276|            <div class="readonly">{incident.notes || ($t('common.na') || '—')}</div>
277|          {/if}
278|        </div>
279|        {#if canManage}
280|          <div class="save-row">
281|            <button class="btn ghost" type="button" onclick={() => void onSave()} disabled={saving}>
282|              <Icon name="save" size={16} />
283|              {saving
284|                ? $t('common.saving') || 'Saving...'
285|                : $t('admin.network.incidents.drawer.save') || 'Save Notes'}
286|            </button>
287|          </div>
288|          <div class="save-row">
289|            <button class="btn ghost" type="button" onclick={onOpenNetworkSettings}>
290|              <Icon name="settings" size={16} />
291|              {$t('admin.network.incidents.drawer.open_network_settings')}
292|            </button>
293|          </div>
294|        {/if}
295|      </div>
296|
297|      <IncidentRunbook
298|        steps={runbookSteps}
299|        {canManage}
300|        onCopyCommand={onCopyRunbookCommand}
301|        onAddStep={onAddRunbookStep}
302|      />
303|
304|      <IncidentTimeline items={activityItems} />
305|    </div>
306|
307|    <div class="drawer-actions">
308|      <button class="btn ghost" type="button" onclick={() => onOpenRouter(incident.router_id)}>
309|        <Icon name="arrow-right" size={16} />
310|        {$t('common.open')}
311|      </button>
312|      {#if incident.status !== 'ack' && incident.status !== 'resolved' && canManage}
313|        <button class="btn ghost" type="button" onclick={() => void onAcknowledge(incident.id)}>
314|          <Icon name="check" size={16} />
315|          {$t('admin.network.alerts.actions.ack')}
316|        </button>
317|      {/if}
318|      {#if incident.status !== 'resolved' && canManage}
319|        <button class="btn ghost" type="button" onclick={() => void onResolve(incident.id)}>
320|          <Icon name="check-circle" size={16} />
321|          {$t('admin.network.alerts.actions.resolve')}
322|        </button>
323|      {/if}
324|    </div>
325|  </aside>
326|{/if}
327|
328|<style>
329|  .drawer-backdrop {
330|    position: fixed;
331|    inset: 0;
332|    border: 0;
333|    background: rgba(0, 0, 0, 0.45);
334|    z-index: 50;
335|  }
336|  .drawer {
337|    position: fixed;
338|    top: 0;
339|    right: 0;
340|    width: min(560px, 92vw);
341|    height: 100vh;
342|    background: var(--bg-surface);
343|    border-left: 1px solid var(--border-color);
344|    z-index: 51;
345|    display: grid;
346|    grid-template-rows: auto 1fr auto;
347|  }
348|  .drawer-head {
349|    padding: 16px;
350|    border-bottom: 1px solid var(--border-color);
351|    display: flex;
352|    align-items: flex-start;
353|    justify-content: space-between;
354|    gap: 12px;
355|  }
356|  .drawer-title {
357|    font-size: 0.78rem;
358|    letter-spacing: 0.08em;
359|    text-transform: uppercase;
360|    color: var(--text-secondary);
361|  }
362|  .drawer-sub {
363|    margin-top: 6px;
364|    font-size: 1.05rem;
365|    font-weight: 900;
366|    color: var(--text-primary);
367|  }
368|  .drawer-body {
369|    padding: 16px;
370|    display: grid;
371|    gap: 14px;
372|    overflow: auto;
373|  }
374|  .detail-grid {
375|    display: grid;
376|    grid-template-columns: repeat(2, minmax(0, 1fr));
377|    gap: 8px;
378|  }
379|  .drow {
380|    border: 1px solid var(--border-color);
381|    border-radius: 10px;
382|    padding: 10px;
383|    display: grid;
384|    gap: 4px;
385|  }
386|  .flag {
387|    display: inline-flex;
388|    align-items: center;
389|    border-radius: 999px;
390|    padding: 3px 9px;
391|    font-size: 0.72rem;
392|    font-weight: 800;
393|    border: 1px solid var(--border-color);
394|  }
395|  .flag-on {
396|    color: var(--color-success);
397|    border-color: color-mix(in srgb, var(--color-success) 45%, var(--border-color));
398|  }
399|  .flag-off {
400|    color: var(--text-secondary);
401|    border-color: var(--border-color);
402|  }
403|  .sla-badge {
404|    display: inline-flex;
405|    align-items: center;
406|    border: 1px solid var(--border-color);
407|    color: var(--text-secondary);
408|    border-radius: 999px;
409|    padding: 2px 8px;
410|    font-size: 0.7rem;
411|    font-weight: 800;
412|    white-space: nowrap;
413|  }
414|  .sla-badge.warn {
415|    color: var(--color-warning);
416|    border-color: color-mix(in srgb, var(--color-warning) 45%, var(--border-color));
417|  }
418|  .sla-badge.breach {
419|    color: var(--color-danger);
420|    border-color: color-mix(in srgb, var(--color-danger) 45%, var(--border-color));
421|  }
422|  .detail-message {
423|    border: 1px solid var(--border-color);
424|    border-radius: 12px;
425|    padding: 12px;
426|    color: var(--text-primary);
427|    line-height: 1.45;
428|    white-space: pre-wrap;
429|  }
430|  .detail-edit {
431|    display: grid;
432|    gap: 10px;
433|    border: 1px solid var(--border-color);
434|    border-radius: 12px;
435|    padding: 12px;
436|    background: color-mix(in srgb, var(--bg-card) 70%, transparent);
437|  }
438|  .impact-card {
439|    display: grid;
440|    gap: 10px;
441|    border: 1px solid var(--border-color);
442|    border-radius: 12px;
443|    padding: 12px;
444|    background: color-mix(in srgb, var(--bg-card) 72%, transparent);
445|  }
446|  .impact-head {
447|    display: flex;
448|    align-items: center;
449|    justify-content: space-between;
450|    gap: 8px;
451|  }
452|  .impact-title {
453|    font-size: 0.84rem;
454|    font-weight: 800;
455|    letter-spacing: 0.02em;
456|    color: var(--text-primary);
457|  }
458|  .impact-count {
459|    min-width: 26px;
460|    height: 20px;
461|    border-radius: 999px;
462|    border: 1px solid var(--border-color);
463|    display: inline-flex;
464|    align-items: center;
465|    justify-content: center;
466|    font-size: 0.72rem;
467|    color: var(--text-secondary);
468|    font-weight: 800;
469|    padding: 0 8px;
470|  }
471|  .impact-list {
472|    display: grid;
473|    gap: 8px;
474|    max-height: 220px;
475|    overflow: auto;
476|  }
477|  .impact-item {
478|    display: flex;
479|    align-items: center;
480|    justify-content: space-between;
481|    gap: 10px;
482|    border: 1px solid var(--border-color);
483|    border-radius: 10px;
484|    padding: 9px 10px;
485|  }
486|  .impact-name {
487|    color: var(--text-primary);
488|    font-weight: 700;
489|    font-size: 0.84rem;
490|  }
491|  .impact-meta {
492|    color: var(--text-secondary);
493|    font-size: 0.75rem;
494|  }
495|  .impact-tags {
496|    display: inline-flex;
497|    align-items: center;
498|    gap: 6px;
499|  }
500|  .impact-tag {
501|    font-size: 0.68rem;
502|    font-weight: 800;
503|    border: 1px solid var(--border-color);
504|    border-radius: 999px;
505|    padding: 3px 8px;
506|    color: var(--text-secondary);
507|    white-space: nowrap;
508|  }
509|  .field {
510|    display: grid;
511|    gap: 6px;
512|  }
513|  .field label {
514|    color: var(--text-secondary);
515|    font-size: 0.8rem;
516|    font-weight: 700;
517|  }
518|  .input,
519|  .textarea {
520|    border: 1px solid var(--border-color);
521|    border-radius: 10px;
522|    background: var(--bg-surface);
523|    color: var(--text-primary);
524|    padding: 10px 12px;
525|    outline: none;
526|  }
527|  .input:focus,
528|  .textarea:focus {
529|    border-color: color-mix(in srgb, var(--accent) 55%, var(--border-color));
530|  }
531|  .readonly {
532|    border: 1px solid var(--border-color);
533|    border-radius: 10px;
534|    background: color-mix(in srgb, var(--bg-surface) 75%, transparent);
535|    color: var(--text-primary);
536|    padding: 10px 12px;
537|    white-space: pre-wrap;
538|  }
539|  .save-row {
540|    display: flex;
541|    justify-content: flex-end;
542|  }
543|  .drawer-actions {
544|    display: flex;
545|    justify-content: flex-end;
546|    gap: 8px;
547|    flex-wrap: wrap;
548|    padding: 12px 16px;
549|    border-top: 1px solid var(--border-color);
550|    background: color-mix(in srgb, var(--bg-card) 80%, transparent);
551|  }
552|  .icon-btn {
553|    width: 34px;
554|    height: 34px;
555|    border-radius: 10px;
556|    border: 1px solid var(--border-color);
557|    background: transparent;
558|    color: var(--text-primary);
559|    display: inline-flex;
560|    align-items: center;
561|    justify-content: center;
562|    cursor: pointer;
563|  }
564|  .icon-btn:hover {
565|    background: var(--bg-hover);
566|  }
567|  @media (max-width: 720px) {
568|    .detail-grid {
569|      grid-template-columns: 1fr;
570|    }
571|  }
572|</style>
573|