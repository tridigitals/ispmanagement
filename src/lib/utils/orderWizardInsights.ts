/**
 * Helper murni wizard order instalasi v2 (gelombang 24c).
 *
 * Validasi tiap langkah + label harga paket dulu inline `$t()` di halaman
 * legacy — kini pesan Indonesia murni + tes.
 */
export type OrderStep1Draft = {
  customerMode: 'new' | 'existing';
  existingCustomerId: string;
  customer: { name: string; email: string; phone: string };
};

export type OrderStep2Draft = {
  locationMode: 'new' | 'existing';
  existingLocationId: string;
  location: { label: string; address_line1: string };
  packageId: string;
};

export function validateOrderStep1(d: OrderStep1Draft): string | null {
  if (d.customerMode === 'existing' && !d.existingCustomerId.trim()) {
    return 'Pilih pelanggan yang sudah ada dulu.';
  }
  if (d.customerMode === 'new') {
    if (!d.customer.name.trim()) return 'Nama pelanggan wajib diisi.';
    if (!d.customer.email.trim() && !d.customer.phone.trim()) {
      return 'Email atau nomor HP wajib diisi salah satu.';
    }
  }
  return null;
}

export function validateOrderStep2(d: OrderStep2Draft): string | null {
  if (d.locationMode === 'existing' && !d.existingLocationId.trim()) {
    return 'Pilih alamat tersimpan dulu.';
  }
  if (d.locationMode === 'new') {
    if (!d.location.label.trim()) return 'Label lokasi wajib diisi.';
    if (!d.location.address_line1.trim()) return 'Alamat baris 1 wajib diisi.';
  }
  if (!d.packageId.trim()) return 'Pilih paket layanan dulu.';
  return null;
}

export function orderPackagePriceLabel(priceMonthly: number, priceYearly: number, cycle: string): string {
  const amount = cycle === 'yearly' && priceYearly > 0 ? priceYearly : priceMonthly;
  return new Intl.NumberFormat('id-ID', { style: 'currency', currency: 'IDR' }).format(amount);
}
