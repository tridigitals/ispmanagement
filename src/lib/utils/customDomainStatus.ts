export type CustomDomainStatus = 'none' | 'pending' | 'active' | 'failed';

export type CustomDomainStatusView = {
  key: CustomDomainStatus;
  label: string;
  tone: 'muted' | 'warning' | 'success' | 'danger';
  description: string;
};

function normalizeStatus(status?: string | null): CustomDomainStatus {
  switch (String(status || '').trim().toLowerCase()) {
    case 'pending':
      return 'pending';
    case 'active':
      return 'active';
    case 'failed':
      return 'failed';
    default:
      return 'none';
  }
}

export function resolveCustomDomainStatusView(args: {
  customDomain?: string | null;
  status?: string | null;
  failureReason?: string | null;
}): CustomDomainStatusView {
  const key = args.customDomain ? normalizeStatus(args.status) : 'none';

  if (key === 'active') {
    return {
      key,
      label: 'Active',
      tone: 'success',
      description: 'Domain ini sudah aktif dan dipakai untuk akses tenant.',
    };
  }

  if (key === 'pending') {
    return {
      key,
      label: 'Pending',
      tone: 'warning',
      description: 'Menunggu verifikasi atau aktivasi sebelum domain bisa dipakai.',
    };
  }

  if (key === 'failed') {
    return {
      key,
      label: 'Failed',
      tone: 'danger',
      description:
        String(args.failureReason || '').trim() || 'Verifikasi domain gagal. Periksa DNS atau konfigurasi domain.',
    };
  }

  return {
    key: 'none',
    label: 'Not set',
    tone: 'muted',
    description: 'Belum ada custom domain yang dikonfigurasi.',
  };
}
