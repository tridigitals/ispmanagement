// ═══ ISPMANAGEMENT UI Revamp — 4 Screen Mockups ═══
// Premium dark, Inter font, flat accent line (no gradient hero),
// custom bottom nav, tinted icon containers, microinteraction hints.

const svg = (path, opts = {}) => {
  const w = opts.w || 24, h = opts.h || 24, sw = opts.sw || 1.8;
  return `<svg width="${w}" height="${h}" viewBox="0 0 24 24" fill="none" stroke="${opts.color || 'currentColor'}" stroke-width="${sw}" stroke-linecap="round" stroke-linejoin="round">${path}</svg>`;
};

// Shared icons
const IC = {
  bell: '<path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/>',
  person: '<circle cx="12" cy="8" r="4"/><path d="M4 20c0-4 4-6 8-6s8 2 8 6"/>',
  settings: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>',
  home: '<path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/>',
  wifi: '<path d="M5 12.55a11 11 0 0 1 14.08 0"/><path d="M1.42 9a16 16 0 0 1 21.16 0"/><path d="M8.53 16.11a6 6 0 0 1 6.95 0"/><line x1="12" y1="20" x2="12.01" y2="20"/>',
  receipt: '<path d="M4 2v20l2-2 2 2 2-2 2 2 2-2 2 2 2-2 2 2V2l-2 2-2-2-2 2-2-2-2 2-2-2z"/><path d="M8 7h8"/><path d="M8 11h8"/><path d="M8 15h5"/>',
  headset: '<path d="M3 18v-6a9 9 0 0 1 18 0v6"/><path d="M21 19a2 2 0 0 1-2 2h-1v-7h3z"/><path d="M3 19a2 2 0 0 0 2 2h1v-7H3z"/>',
  chevron: '<polyline points="9 18 15 12 9 6"/>',
  router: '<rect x="2" y="14" width="20" height="8" rx="2"/><path d="M6.5 18.5h.01"/><path d="M10 18.5h.01"/><path d="M14 18.5h.01"/><path d="M17.5 18.5h.01"/><path d="M12 14v-4"/><path d="M8 10V7"/><path d="M16 10V7"/><path d="M12 7V3"/>',
  lock: '<rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
  eye: '<path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7z"/><circle cx="12" cy="12" r="3"/>',
  eyeOff: '<path d="M9.88 9.88a3 3 0 1 0 4.24 4.24"/><path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68"/><path d="M6.61 6.61A13.526 13.526 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61"/><line x1="2" y1="2" x2="22" y2="22"/>',
  fingerprint: '<path d="M12 11a2 2 0 0 0-2 2c0 1.5 1 3 1 5"/><path d="M12 11a2 2 0 0 1 2 2c0 4-3 6-3 10"/><path d="M17 18.5a7 7 0 0 0-1-3.5c-.5-1-1-2-1-4"/><path d="M6.5 5.5A7 7 0 0 1 19 13c0 1.5-.5 3-1 4.5"/>',
  download: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>',
  upload: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>',
  help: '<circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/>',
  edit: '<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4z"/>',
  logout: '<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/>',
  moon: '<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>',
  sun: '<circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>',
  auto: '<circle cx="12" cy="12" r="11"/><path d="M12 2v10l5.5 5.5"/>',
  signal: '<path d="M2 20h.01"/><path d="M7 20v-4"/><path d="M12 20v-8"/><path d="M17 20V8"/><path d="M22 4v16"/>',
  plus: '<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>',
  arrow: '<line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/>',
};

// ═══════════════════════════════════════════════════════════════
// SCREEN 1: HOME
// ═══════════════════════════════════════════════════════════════
function renderHome() {
  return `
  <div class="content" style="padding-bottom:100px;">
    <!-- AppBar -->
    <div class="appbar">
      <div class="appbar-greeting">Hai, Tri <span class="wave">👋</span></div>
      <div class="appbar-actions">
        <div class="icon-btn">${svg(IC.bell)}<span class="badge">3</span></div>
        <div class="icon-btn">${svg(IC.person)}</div>
        <div class="icon-btn">${svg(IC.settings)}</div>
      </div>
    </div>

    <div style="padding:0 20px;">
      <!-- Hero subscription card -->
      <div class="card" style="margin-top:8px;transition:transform 0.15s;" onmouseover="this.style.transform='scale(1.01)'" onmouseout="this.style.transform='scale(1)'">
        <div class="accent-line"></div>
        <div style="padding:20px;">
          <div style="display:flex;justify-content:space-between;align-items:flex-start;">
            <div style="text-transform:uppercase;font-size:11px;font-weight:600;letter-spacing:1px;color:var(--text-secondary);flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">FIBER 50MBPS</div>
            <span class="badge success">Aktif</span>
          </div>
          <div style="height:24px;"></div>
          <div style="font-size:42px;font-weight:800;letter-spacing:-1.5px;line-height:1;">Rp 350.000</div>
          <div style="font-size:13px;color:var(--text-muted);margin-top:4px;">/ bulan</div>
          <div style="height:20px;"></div>
          <div style="display:flex;align-items:center;gap:10px;">
            <div style="width:32px;height:32px;border-radius:10px;background:var(--accent-surface);display:flex;align-items:center;justify-content:center;">
              <span style="color:var(--accent);">${svg(IC.router,{w:16,h:16,color:'var(--accent)'})}</span>
            </div>
            <div style="flex:1;font-size:14px;color:var(--text-secondary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">Router Semarang — Cabang A</div>
            ${svg(IC.chevron,{w:22,h:22,color:'var(--text-muted)'})}
          </div>
        </div>
      </div>

      <div style="height:20px;"></div>

      <!-- Network status -->
      <div style="display:flex;align-items:center;gap:10px;padding:12px 16px;background:var(--success-surface);border-radius:var(--radius-lg);border:0.5px solid rgba(0,210,160,0.15);">
        <div style="width:36px;height:36px;border-radius:10px;background:rgba(0,210,160,0.15);display:flex;align-items:center;justify-content:center;color:var(--success);">
          ${svg(IC.signal,{w:18,h:18,color:'var(--success)',sw:2.5})}
        </div>
        <div style="flex:1;">
          <div style="font-size:13px;font-weight:600;color:var(--text-primary);">Jaringan Normal</div>
          <div style="font-size:11px;color:var(--text-muted);">Latensi 12ms · Uptime 99.8%</div>
        </div>
        <span class="badge success" style="font-size:10px;">Online</span>
      </div>

      <div style="height:20px;"></div>

      <!-- Announcement -->
      <div style="display:flex;align-items:center;gap:10px;padding:12px 16px;background:var(--accent-surface);border-radius:var(--radius-lg);">
        <div style="font-size:20px;">📢</div>
        <div style="flex:1;font-size:13px;color:var(--accent-text);font-weight:500;">Maintenance terjadwal 15 Jul 02:00–04:00 WIB</div>
        ${svg(IC.chevron,{w:18,h:18,color:'var(--accent-text)'})}
      </div>

      <div style="height:24px;"></div>

      <!-- Recent invoices -->
      <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px;">
        <div style="font-size:17px;font-weight:700;">Tagihan Terkini</div>
        <span style="font-size:13px;font-weight:600;color:var(--accent);cursor:pointer;">Lihat Semua</span>
      </div>

      <div class="card">
        <!-- Invoice item -->
        ${[1,2,3].map((i) => {
          const statuses = [{l:'Lunas',t:'success'},{l:'Jatuh Tempo',t:'danger'},{l:'Menunggu',t:'warning'}];
          const s = statuses[i-1];
          const amounts = ['Rp 350.000','Rp 350.000','Rp 350.000'];
          return `
          <div style="display:flex;align-items:center;padding:14px 16px;border-bottom:0.5px solid var(--border-subtle);">
            <div style="width:36px;height:36px;border-radius:10px;background:var(--surface-tertiary);display:flex;align-items:center;justify-content:center;flex-shrink:0;">
              ${svg(IC.receipt,{w:16,h:16,color:'var(--text-secondary)'})}
            </div>
            <div style="margin-left:12px;flex:1;min-width:0;">
              <div style="font-size:14px;font-weight:600;color:var(--text-primary);">INV-2026-07-0${i}</div>
              <div style="font-size:12px;color:var(--text-muted);margin-top:2px;">Fiber 50Mbps · Jul 2026</div>
            </div>
            <div style="text-align:right;flex-shrink:0;">
              <div style="font-size:14px;font-weight:600;color:var(--text-primary);">${amounts[i-1]}</div>
              <div style="margin-top:4px;"><span class="badge ${s.t}">${s.l}</span></div>
            </div>
          </div>
          `;
        }).join('')}
      </div>
    </div>

    <!-- Bottom Nav -->
    <div style="position:absolute;bottom:0;left:0;right:0;background:var(--surface);border-top:0.5px solid var(--border-subtle);padding:6px 4px 20px;">
      <div style="display:flex;gap:0;">
        ${[
          {ic:IC.home,l:'Beranda',active:true},
          {ic:IC.wifi,l:'Layanan'},
          {ic:IC.receipt,l:'Tagihan'},
          {ic:IC.headset,l:'Support'},
        ].map(t => `
          <div style="flex:1;display:flex;flex-direction:column;align-items:center;gap:2px;padding:6px 0;border-radius:var(--radius-pill);${t.active?'background:rgba(108,92,231,0.12);':''}">
            ${svg(t.ic,{w:22,h:22,color:t.active?'var(--accent)':'var(--text-muted)',sw:t.active?2.2:1.8})}
            <span style="font-size:10px;font-weight:${t.active?'600':'400'};color:${t.active?'var(--accent)':'var(--text-muted)'};">${t.l}</span>
          </div>
        `).join('')}
      </div>
    </div>
  </div>
  `;
}

// ═══════════════════════════════════════════════════════════════
// SCREEN 2: LOGIN
// ═══════════════════════════════════════════════════════════════
function renderLogin() {
  return `
  <div class="content" style="display:flex;flex-direction:column;align-items:center;justify-content:center;padding:40px;min-height:calc(812px - 44px);">
    <!-- Logo -->
    <div style="width:72px;height:72px;border-radius:20px;background:var(--accent-surface);display:flex;align-items:center;justify-content:center;margin-bottom:24px;">
      <span style="font-size:32px;font-weight:800;color:var(--accent);letter-spacing:-2px;">IS</span>
    </div>
    <div style="font-size:26px;font-weight:800;letter-spacing:-0.5px;margin-bottom:6px;">Selamat Datang</div>
    <div style="font-size:14px;color:var(--text-muted);margin-bottom:36px;text-align:center;">Masuk untuk mengelola layanan internet Anda</div>

    <!-- Input: identifier -->
    <div style="width:100%;margin-bottom:12px;">
      <div style="font-size:11px;font-weight:600;letter-spacing:0.5px;text-transform:uppercase;color:var(--text-muted);margin-bottom:8px;">Email atau Nomor HP</div>
      <div style="background:var(--surface);border:1px solid var(--border-subtle);border-radius:var(--radius-md);padding:14px 16px;display:flex;align-items:center;gap:10px;transition:border 0.2s;">
        ${svg(IC.person,{w:18,h:18,color:'var(--text-muted)'})}
        <span style="font-size:15px;color:var(--text-primary);">tri@tridigitals.com</span>
      </div>
    </div>

    <!-- Input: password -->
    <div style="width:100%;margin-bottom:8px;">
      <div style="font-size:11px;font-weight:600;letter-spacing:0.5px;text-transform:uppercase;color:var(--text-muted);margin-bottom:8px;">Kata Sandi</div>
      <div style="background:var(--surface);border:1px solid var(--border-subtle);border-radius:var(--radius-md);padding:14px 16px;display:flex;align-items:center;gap:10px;">
        ${svg(IC.lock,{w:18,h:18,color:'var(--text-muted)'})}
        <span style="font-size:15px;color:var(--text-primary);flex:1;letter-spacing:4px;">••••••••</span>
        ${svg(IC.eye,{w:18,h:18,color:'var(--text-muted)'})}
      </div>
    </div>

    <!-- Forgot password -->
    <div style="width:100%;text-align:right;margin-bottom:24px;">
      <span style="font-size:13px;color:var(--accent);font-weight:600;cursor:pointer;">Lupa Kata Sandi?</span>
    </div>

    <!-- Login button -->
    <button style="width:100%;padding:15px;background:var(--accent);color:white;border:none;border-radius:var(--radius-md);font-size:15px;font-weight:700;font-family:var(--font);cursor:pointer;transition:transform 0.15s;" onmouseover="this.style.transform='scale(0.98)'" onmouseout="this.style.transform='scale(1)'">
      Masuk
    </button>

    <!-- Divider -->
    <div style="display:flex;align-items:center;gap:12px;width:100%;margin:24px 0;">
      <div style="flex:1;height:1px;background:var(--border-subtle);"></div>
      <span style="font-size:12px;color:var(--text-muted);">atau</span>
      <div style="flex:1;height:1px;background:var(--border-subtle);"></div>
    </div>

    <!-- Biometric button -->
    <button style="width:100%;padding:14px;background:transparent;border:1px solid var(--border);border-radius:var(--radius-md);font-size:14px;font-weight:600;color:var(--text-primary);font-family:var(--font);display:flex;align-items:center;justify-content:center;gap:10px;cursor:pointer;">
      ${svg(IC.fingerprint,{w:22,h:22,color:'var(--accent)',sw:2})}
      Masuk dengan Sidik Jari
    </button>

    <div style="flex:1;"></div>
    <div style="font-size:13px;color:var(--text-muted);margin-top:24px;">Belum punya akun? <span style="color:var(--accent);font-weight:600;">Hubungi ISP</span></div>
  </div>
  `;
}

// ═══════════════════════════════════════════════════════════════
// SCREEN 3: INVOICE DETAIL
// ═══════════════════════════════════════════════════════════════
function renderInvoice() {
  return `
  <div class="content" style="padding-bottom:40px;">
    <!-- AppBar -->
    <div class="appbar">
      <div class="icon-btn">${svg('<polyline points="15 18 9 12 15 6"/>',{w:22,h:22,color:'var(--text-secondary)'})}</div>
      <div style="font-size:17px;font-weight:600;">Detail Tagihan</div>
      <div class="icon-btn">${svg(IC.download,{w:20,h:20,color:'var(--text-secondary)'})}</div>
    </div>

    <div style="padding:0 20px;">
      <!-- Hero card — FLAT, accent line (replaces gradient) -->
      <div class="card" style="margin-top:8px;">
        <div class="accent-line" style="background:var(--danger);"></div>
        <div style="padding:24px 20px;">
          <div style="display:flex;justify-content:space-between;align-items:center;">
            <div style="font-size:13px;font-weight:500;color:var(--text-secondary);">INV-2026-07-002</div>
            <span class="badge danger">Jatuh Tempo</span>
          </div>
          <div style="height:20px;"></div>
          <div style="font-size:36px;font-weight:800;letter-spacing:-1.5px;line-height:1;">Rp 350.000</div>
          <div style="font-size:13px;color:var(--text-muted);margin-top:6px;">Jatuh tempo 15 Juli 2026</div>
        </div>
      </div>

      <div style="height:16px;"></div>

      <!-- Info card -->
      <div class="flat-card">
        <div style="font-size:13px;font-weight:600;color:var(--text-secondary);margin-bottom:12px;">Informasi Tagihan</div>
        <div style="display:flex;justify-content:space-between;padding:8px 0;">
          <span style="font-size:13px;color:var(--text-muted);">Layanan</span>
          <span style="font-size:13px;font-weight:500;color:var(--text-primary);">Fiber 50Mbps</span>
        </div>
        <div style="display:flex;justify-content:space-between;padding:8px 0;">
          <span style="font-size:13px;color:var(--text-muted);">Periode</span>
          <span style="font-size:13px;font-weight:500;color:var(--text-primary);">Juli 2026</span>
        </div>
        <div style="display:flex;justify-content:space-between;padding:8px 0;">
          <span style="font-size:13px;color:var(--text-muted);">Jatuh Tempo</span>
          <span style="font-size:13px;font-weight:500;color:var(--text-primary);">15 Jul 2026</span>
        </div>
        <div style="display:flex;justify-content:space-between;padding:8px 0;border-top:0.5px solid var(--border-subtle);margin-top:8px;padding-top:12px;">
          <span style="font-size:14px;font-weight:600;color:var(--text-primary);">Total</span>
          <span style="font-size:18px;font-weight:800;color:var(--text-primary);">Rp 350.000</span>
        </div>
      </div>

      <div style="height:16px;"></div>

      <!-- Actions -->
      <button style="width:100%;padding:15px;background:var(--accent);color:white;border:none;border-radius:var(--radius-md);font-size:15px;font-weight:700;font-family:var(--font);display:flex;align-items:center;justify-content:center;gap:8px;cursor:pointer;">
        ${svg(IC.receipt,{w:18,h:18,color:'white',sw:2})}
        Bayar Sekarang
      </button>
      <div style="height:8px;"></div>
      <button style="width:100%;padding:14px;background:transparent;border:1px solid var(--border);border-radius:var(--radius-md);font-size:14px;font-weight:600;color:var(--text-primary);font-family:var(--font);display:flex;align-items:center;justify-content:center;gap:8px;cursor:pointer;">
        ${svg(IC.upload,{w:18,h:18,color:'var(--text-secondary)'})}
        Upload Bukti Pembayaran
      </button>
      <div style="height:8px;"></div>
      <button style="width:100%;padding:14px;background:transparent;border:none;border-radius:var(--radius-md);font-size:14px;font-weight:600;color:var(--accent);font-family:var(--font);display:flex;align-items:center;justify-content:center;gap:8px;cursor:pointer;">
        ${svg(IC.help,{w:18,h:18,color:'var(--accent)'})}
        Butuh Bantuan?
      </button>
    </div>
  </div>
  `;
}

// ═══════════════════════════════════════════════════════════════
// SCREEN 4: PROFILE
// ═══════════════════════════════════════════════════════════════
function renderProfile() {
  const items = [
    {ic:IC.edit,l:'Edit Profil',tint:'accent'},
    {ic:IC.help,l:'FAQ',tint:'info'},
    {ic:IC.headset,l:'Hubungi Kami',tint:'success'},
    {ic:IC.bell,l:'Notifikasi',tint:'warning',badge:'3'},
    {ic:IC.lock,l:'Ubah Kata Sandi',tint:'neutral'},
  ];
  const logoutItem = {ic:IC.logout,l:'Keluar',tint:'danger'};

  return `
  <div class="content" style="padding-bottom:40px;">
    <div class="appbar">
      <div style="font-size:22px;font-weight:700;letter-spacing:-0.3px;">Profil</div>
      <div class="appbar-actions">
        <div class="icon-btn">${svg(IC.settings)}</div>
      </div>
    </div>

    <div style="padding:0 20px;">
      <!-- Avatar block -->
      <div style="text-align:center;margin-top:24px;margin-bottom:24px;">
        <div style="width:88px;height:88px;border-radius:50%;background:var(--accent-surface);border:2px solid var(--accent);margin:0 auto;display:flex;align-items:center;justify-content:center;">
          <span style="font-size:36px;font-weight:800;color:var(--accent);">TY</span>
        </div>
        <div style="font-size:18px;font-weight:700;margin-top:12px;">Tri Yanto</div>
        <div style="font-size:13px;color:var(--text-muted);margin-top:2px;">tri@tridigitals.com</div>
        <div style="margin-top:8px;">
          <span class="badge info" style="font-size:11px;">Premium Member</span>
        </div>
      </div>

      <!-- Profile items group -->
      <div class="card" style="margin-bottom:12px;">
        ${items.map(item => `
          <div style="display:flex;align-items:center;padding:14px 16px;border-bottom:0.5px solid var(--border-subtle);">
            <div style="width:36px;height:36px;border-radius:10px;background:var(--surface-tertiary);display:flex;align-items:center;justify-content:center;flex-shrink:0;">
              ${svg(item.ic,{w:18,h:18,color:`var(--${item.tint === 'neutral' ? 'text-secondary' : item.tint})`})}
            </div>
            <div style="flex:1;margin-left:12px;font-size:14px;font-weight:500;color:var(--text-primary);">${item.l}</div>
            ${item.badge ? `<span class="badge danger" style="font-size:10px;">${item.badge}</span>` : ''}
            ${svg(IC.chevron,{w:20,h:20,color:'var(--text-muted)'})}
          </div>
        `).join('')}
      </div>

      <!-- Theme toggle card -->
      <div class="card" style="margin-bottom:12px;padding:14px 16px;">
        <div style="display:flex;align-items:center;gap:12px;">
          <div style="width:36px;height:36px;border-radius:10px;background:var(--surface-tertiary);display:flex;align-items:center;justify-content:center;">
            ${svg(IC.moon,{w:18,h:18,color:'var(--accent)'})}
          </div>
          <div style="flex:1;font-size:14px;font-weight:500;">Tema Tampilan</div>
          <div style="display:flex;gap:4px;background:var(--surface-tertiary);border-radius:var(--radius-pill);padding:3px;">
            <div style="padding:7px;border-radius:50%;cursor:pointer;">${svg(IC.sun,{w:16,h:16,color:'var(--text-muted)'})}</div>
            <div style="padding:7px;border-radius:50%;background:var(--accent);cursor:pointer;">${svg(IC.auto,{w:16,h:16,color:'var(--text-muted)'})}</div>
            <div style="padding:7px;border-radius:50%;background:var(--accent);cursor:pointer;">${svg(IC.moon,{w:16,h:16,color:'white'})}</div>
          </div>
        </div>
      </div>

      <!-- Logout -->
      <div class="card" style="margin-bottom:12px;">
        <div style="display:flex;align-items:center;padding:14px 16px;">
          <div style="width:36px;height:36px;border-radius:10px;background:var(--danger-surface);display:flex;align-items:center;justify-content:center;flex-shrink:0;">
            ${svg(IC.logout,{w:18,h:18,color:'var(--danger)'})}
          </div>
          <div style="flex:1;margin-left:12px;font-size:14px;font-weight:600;color:var(--danger);">Keluar</div>
        </div>
      </div>

      <!-- Version -->
      <div style="text-align:center;font-size:12px;color:var(--text-muted);margin-top:16px;">v0.1.0+64 · ISPMANAGEMENT Customer</div>
    </div>
  </div>
  `;
}

// ═══ Render ═══
document.getElementById('home').innerHTML = renderHome();
document.getElementById('login').innerHTML = renderLogin();
document.getElementById('invoice').innerHTML = renderInvoice();
document.getElementById('profile').innerHTML = renderProfile();
