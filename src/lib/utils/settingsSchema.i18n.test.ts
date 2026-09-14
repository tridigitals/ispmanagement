import { describe, it, expect } from 'vitest';
import { applyLabels, applyPanelLabels, isVisible, SETTING_SECTIONS, validate } from './settingsSchema';
import id from '../i18n/namespaces/id/admin.json';
import en from '../i18n/namespaces/en/admin.json';

describe('settings_v2 schema i18n', () => {
  const s = (ns: any) => ns.settings_v2.schema;

  it('id & en punya key section lengkap', () => {
    for (const sec of SETTING_SECTIONS) {
      for (const [loc, ns] of [['id', id], ['en', en]] as const) {
        expect(s(ns).sections?.[sec.id]?.label, `${loc}/${sec.id}`).toBeTruthy();
        expect(s(ns).sections?.[sec.id]?.desc, `${loc}/${sec.id}`).toBeTruthy();
      }
    }
  });

  it('id & en punya label untuk SETIAP field', () => {
    for (const sec of SETTING_SECTIONS) {
      for (const f of sec.fields) {
        for (const [loc, ns] of [['id', id], ['en', en]] as const) {
          expect(s(ns).fields?.[f.key]?.label, `${loc}/${f.key}`).toBeTruthy();
        }
      }
    }
  });

  it('setiap option select punya terjemahan', () => {
    for (const sec of SETTING_SECTIONS) {
      for (const f of sec.fields.filter((x) => x.options)) {
        for (const o of f.options!) {
          for (const [loc, ns] of [['id', id], ['en', en]] as const) {
            expect(s(ns).fields?.[f.key]?.options?.[o.value], `${loc}/${f.key}/${o.value}`).toBeTruthy();
          }
        }
      }
    }
  });

  it('suffix satuan punya terjemahan', () => {
    const suffixes = new Set<string>();
    for (const sec of SETTING_SECTIONS)
      for (const f of sec.fields) if (f.suffix) suffixes.add(f.suffix);
    expect(suffixes.size).toBeGreaterThan(0);
    for (const [loc, ns] of [['id', id], ['en', en]] as const) {
      const units = s(ns).units;
      for (const u of suffixes) expect(units?.[u], `${loc}/${u}`).toBeTruthy();
    }
  });

  it('applyLabels menimpa label & option saat kamus diberikan', () => {
    const sections = applyLabels({
      fields: { app_name: { label: 'App name' } },
    });
    expect(sections[0].fields[0].label).toBe('App name');
    expect(sections[0].fields[1].label).toBe('Deskripsi'); // default id tetap
  });

  it('applyPanelLabels lengkap utk semua panel', () => {
    for (const [loc, ns] of [['id', id], ['en', en]] as const) {
      const panels = applyPanelLabels(s(ns).panels);
      for (const p of panels) {
        expect(p.label, `${loc}/${p.id}`).toBeTruthy();
        expect(p.desc, `${loc}/${p.id}`).toBeTruthy();
        // harus berbeda dari default hanya kalau memang sudah diterjemahkan
      }
    }
    // EN harus benar-benar bahasa Inggris utk minimal branding
    const enPanels = applyPanelLabels(s(en).panels);
    expect(enPanels.find((p) => p.id === 'branding')?.label).toMatch(/Brand/i);
  });

  it('field bersyarat (whenOneOf) ikut terdeteksi isVisible', () => {
    const f = SETTING_SECTIONS.flatMap((x) => x.fields).find((x) => x.whenOneOf);
    expect(f).toBeTruthy();
    const driver = f!.whenOneOf!.key;
    const vis = f!.whenOneOf!;expect(isVisible(f!, { [driver]: 'system' })).toBe(false);
    for (const v of f!.whenOneOf!.equals) expect(isVisible(f!, { [driver]: v })).toBe(true);
  });
});
