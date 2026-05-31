# Plan: Polish UI `/superadmin` — konsistensi + mobile responsive

## Status: ✅ SELESAI (verified 2026-05-31)
Audit ulang menunjukkan semua fase sudah terimplementasi di codebase. Detail per fase di bawah.

## Konteks
Audit 13 halaman superadmin + layout. Mayoritas sudah rapi (token CSS, breakpoint,
light/dark). Beberapa halaman menyimpang dari konvensi (Tailwind utility, hardcoded
text, grid tidak responsif). Target: samakan ke pola `superadmin-content` + scoped CSS
pakai design tokens, dan pastikan semua responsif di mobile + light/dark.

## Konvensi project (acuan)
- Container: `.superadmin-content { padding: clamp(16px,3vw,32px); max-width:1400px; margin:0 auto }`
- Warna via `var(--*)` token, `color-mix(in srgb, var(--color-*) N%, ...)`
- Light mode override: `:global([data-theme='light']) .xxx { ... }`
- Modal: `$lib/components/ui/Modal.svelte` (bindable show, snippet children/footer, width prop)
- Select: `$lib/components/ui/Select.svelte` (bindable value, options [{label,value}])
- i18n DUAL-FILE: `namespaces/{id,en}/superadmin.json` (runtime) + `locales/{id,en}.json` (lint)
- Mobile breakpoint umum: 720px (cards), 900px (layout)

## Fase 1 — registration-approvals (REWORK TOTAL) ✅
File: `src/routes/superadmin/registration-approvals/+page.svelte`
- [x] Scoped CSS + token (0 Tailwind utility)
- [x] Wrapper `.superadmin-content fade-in` + card pattern
- [x] Pakai `Modal.svelte` (approve + reject dialog)
- [x] data-table pattern + state (loading/empty/error)
- [x] i18n semua label

## Fase 2 — users stats-row mobile ✅
File: `src/routes/superadmin/users/+page.svelte`
- [x] `.stats-row` repeat(4) → `@media (max-width:900px){repeat(2)}` + `(max-width:480px){1fr}`

## Fase 3 — system page ✅
File: `src/routes/superadmin/system/+page.svelte`
- [x] `.page-container padding` → `clamp(16px,3vw,32px)` + max-width 1400px + margin auto
- [x] grid → `repeat(auto-fit, minmax(min(100%,360px),1fr))`
- [x] `@media (max-width:720px)` ada

## Fase 4 — backups polish ✅
File: `src/routes/superadmin/backups/+page.svelte`
- [x] `Actions` pakai `$t('common.actions') || 'Actions'` (fallback OK)
- [x] `flex/justify-end/gap-2` BUKAN Tailwind — scoped CSS lokal (baris 513-521). Tidak perlu diubah.

## Fase 5 — Verify ✅
- [x] `npm run check` (svelte-check) — 0 errors, 17 warnings (pre-existing, bukan dari fase ini)
- [x] `npm run i18n:check` — 0 missing key (en + id)
- [ ] Visual probe dev server (opsional — bisa dilakukan saat ada perubahan visual baru)

## Pitfalls (dari skill isp-management-dev)
- i18n: update namespace file DULU (runtime), baru locales (lint). Verify both via search.
- Tidak ada template-literal hardcoded — semua string lewat `$t() || 'fallback'`.
- Jangan campur cargo fmt / formatter noise ke commit feature.
