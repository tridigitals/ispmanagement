# Network Reusable Components TODO

Tujuan: kurangi duplikasi markup/CSS di halaman network agar maintenance lebih cepat dan UI konsisten.

## Sudah dikerjakan

- [x] Buat komponen `src/lib/components/network/NetworkFilterPanel.svelte`
- [x] Pakai `NetworkFilterPanel` di:
  - `src/routes/[tenant]/(app)/admin/network/noc/+page.svelte`
  - `src/routes/[tenant]/(app)/admin/network/alerts/+page.svelte`
  - `src/routes/[tenant]/(app)/admin/network/incidents/+page.svelte`
- [x] Buat komponen `src/lib/components/network/NetworkPageHeader.svelte`
- [x] Pakai `NetworkPageHeader` di:
  - `src/routes/[tenant]/(app)/admin/network/noc/+page.svelte`
  - `src/routes/[tenant]/(app)/admin/network/alerts/+page.svelte`
  - `src/routes/[tenant]/(app)/admin/network/incidents/+page.svelte`
  - `src/routes/[tenant]/(app)/admin/network/routers/+page.svelte`
  - `src/routes/[tenant]/(app)/admin/network/logs/+page.svelte`
  - `src/routes/[tenant]/(app)/admin/network/routers/[id]/+page.svelte`
- [x] Satukan konstanta/opsi wallboard ke shared constant:
  - `src/lib/constants/wallboard.ts`
  - dipakai di:
    - `src/routes/[tenant]/(app)/admin/network/noc/wallboard/+page.svelte`
    - `src/routes/[tenant]/(app)/admin/network/noc/wallboard/settings/+page.svelte`
- [x] Ekstrak panel controls insights wallboard jadi komponen reusable:
  - `src/lib/components/network/WallboardInsightsControls.svelte`
  - dipakai di:
    - `src/routes/[tenant]/(app)/admin/network/noc/wallboard/+page.svelte`
- [x] Ekstrak panel summary insights wallboard (SLO + Top Issues + Incidents + Timeline):
  - `src/lib/components/network/WallboardInsightsSummary.svelte`
  - dipakai di:
    - `src/routes/[tenant]/(app)/admin/network/noc/wallboard/+page.svelte`
- [x] Ekstrak panel alert floating wallboard:
  - `src/lib/components/network/WallboardAlertsPanel.svelte`
  - dipakai di:
    - `src/routes/[tenant]/(app)/admin/network/noc/wallboard/+page.svelte`
- [x] Ekstrak util/helpers wallboard ke modul terpisah:
  - `src/lib/components/network/wallboardUtils.ts`
  - `src/lib/components/network/wallboardDerivations.ts`
  - `src/lib/components/network/wallboardAlertsActions.ts`
  - `src/lib/components/network/wallboardMetrics.ts`
  - `src/lib/components/network/wallboardMetricsSelection.ts`
  - `src/lib/components/network/wallboardConfig.ts`

## Next prioritas tinggi

- [ ] Lanjutkan adopsi `NetworkPageHeader` untuk area judul/subtitle + action bar
  - Kandidat:
    - `src/routes/[tenant]/(app)/admin/network/noc/wallboard/+page.svelte` (bagian panel/header modal)

- [x] Buat komponen `RowActionButtons` (open/ack/resolve/snooze pattern)
  - Kandidat:
    - `src/routes/[tenant]/(app)/admin/network/alerts/+page.svelte`
    - `src/routes/[tenant]/(app)/admin/network/incidents/+page.svelte`

- [x] Buat util/export helper untuk CSV/Excel
  - Kandidat:
    - `src/routes/[tenant]/(app)/admin/network/incidents/+page.svelte`
    - `src/routes/[tenant]/(app)/admin/network/noc/wallboard/+page.svelte`

## Next prioritas menengah

- [ ] Satukan style token table-wrapper/card surface ke utility class global
  - Kandidat:
    - `src/routes/[tenant]/(app)/admin/network/*/+page.svelte`

- [ ] Pecah drawer/detail incident jadi komponen terpisah
  - Kandidat:
    - `src/routes/[tenant]/(app)/admin/network/incidents/+page.svelte`
  - Rencana komponen:
      - [x] `IncidentDetailDrawer.svelte`
      - [x] `IncidentRunbook.svelte`
      - [x] `IncidentTimeline.svelte`

- [x] Pecah simulate incident drawer jadi komponen reusable
  - `src/lib/components/network/IncidentSimulateDrawer.svelte`
  - dipakai di:
    - `src/routes/[tenant]/(app)/admin/network/incidents/+page.svelte`

- [ ] Satukan badge/status pill ke komponen reusable kecil
  - Kandidat:
    - `alerts`, `incidents`, `noc`, `routers/[id]`

## Wallboard Refactor Roadmap (Batch Besar)

- [ ] 3. Ekstrak polling engine live metrics per-router
  - Target file baru:
    - `src/lib/components/network/wallboardPolling.ts`
  - Scope:
    - batching router/interface
    - retry/backoff state
    - update `liveRates` + `series`
    - interface data contract typed

- [ ] 4. Ekstrak drag/drop dan tile interaction state
  - Target file baru:
    - `src/lib/components/network/wallboardDnD.ts`
  - Scope:
    - start/move/end drag
    - swap tile
    - guard slot index + bounds

- [ ] 5. Ekstrak full modal tabs (live/metrics) controller
  - Target file baru:
    - `src/lib/components/network/wallboardFullPanel.ts`
  - Scope:
    - state transisi tab
    - open/close/reset behavior
    - metric range refresh trigger

- [x] 6. Pisahkan visual tile menjadi komponen Svelte
  - Target file baru:
    - [x] `src/lib/components/network/WallboardInterfaceTile.svelte`
  - Scope:
    - [x] head/meta/status badge
    - [x] spark mini chart + hover tooltip
    - [x] action menu per tile

- [x] 7. Pisahkan modal picker/edit threshold jadi komponen
  - Target file baru:
    - [x] `src/lib/components/network/WallboardSlotPicker.svelte`
    - [x] `src/lib/components/network/WallboardThresholdDialog.svelte`
  - Scope:
    - [x] router+iface picker
    - [x] threshold parsing + validation

- [ ] 8. Tambah test untuk helper wallboard
  - Target file baru:
    - `src/lib/components/network/wallboardMetrics.test.ts`
    - `src/lib/components/network/wallboardConfig.test.ts`
    - `src/lib/components/network/wallboardDerivations.test.ts`
  - Scope:
    - parse/aggregate edge cases
    - config load/save compatibility
    - sorting/filter deterministic behavior

## Catatan

- Fokus awal tetap di domain network karena duplikasi paling tinggi ada di sana.
- Setelah stabil, pattern yang sama bisa dipakai ke modul non-network.
