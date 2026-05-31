# Plan: Polish UI `/superadmin` — konsistensi + mobile responsive

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

## Fase 1 — registration-approvals (REWORK TOTAL) 🔴
File: `src/routes/superadmin/registration-approvals/+page.svelte`
- [ ] Buang semua utility Tailwind → scoped CSS + token
- [ ] Wrapper `.superadmin-content fade-in` + card pattern
- [ ] Ganti 2 modal manual → `Modal.svelte`
- [ ] Approve dialog: `Select` tenant (dari `superadmin.listTenants()`) + `Select` role (dari `roles.list()`)
- [ ] i18n semua label (Name/Email/Registered/Actions/Cancel/placeholder/loading/empty)
- [ ] Mobile: card view < 720px, table ≥ 720px
- [ ] Tambah i18n key baru ke 4 file (namespaces id/en + locales id/en)

## Fase 2 — users stats-row mobile 🔴
File: `src/routes/superadmin/users/+page.svelte`
- [ ] `.stats-row` repeat(4) → `@media (max-width:720px){repeat(2,1fr)}` + `(max-width:420px){1fr}`

## Fase 3 — system page 🔴
File: `src/routes/superadmin/system/+page.svelte`
- [ ] `.page-container padding: 2rem` → `clamp(16px,3vw,32px)`; align `superadmin-content`
- [ ] `.grid-2 minmax(400px,1fr)` → `minmax(min(100%,360px),1fr)`
- [ ] `@media (max-width:720px)` header stack vertical, view-toggle full-width

## Fase 4 — backups polish 🟡
File: `src/routes/superadmin/backups/+page.svelte`
- [ ] i18n hardcoded `Actions` + toast literal
- [ ] Hapus utility Tailwind sisa (cek dulu sebelum ubah)

## Fase 5 — Verify
- [ ] `npm run check` (svelte-check) — 0 new error
- [ ] `npm run i18n:check` — 0 new missing key
- [ ] Visual probe dev server (mobile viewport + light/dark)

## Pitfalls (dari skill isp-management-dev)
- i18n: update namespace file DULU (runtime), baru locales (lint). Verify both via search.
- Tidak ada template-literal hardcoded — semua string lewat `$t() || 'fallback'`.
- Jangan campur cargo fmt / formatter noise ke commit feature.
