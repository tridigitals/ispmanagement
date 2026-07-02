import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Indonesian (`id`).
class AppLocalizationsId extends AppLocalizations {
  AppLocalizationsId([String locale = 'id']) : super(locale);

  @override
  String get appTitle => 'ISP Teknisi';

  @override
  String get home => 'Beranda';

  @override
  String get profile => 'Profil';

  @override
  String get settings => 'Pengaturan';

  @override
  String get support => 'Bantuan';

  @override
  String get login => 'Masuk';

  @override
  String get logout => 'Keluar';

  @override
  String get register => 'Daftar';

  @override
  String get createAccount => 'Buat akun';

  @override
  String get email => 'Email';

  @override
  String get phone => 'No. HP';

  @override
  String get password => '***';

  @override
  String get currentPassword => 'Kata sandi saat ini';

  @override
  String get newPassword => 'Kata sandi baru';

  @override
  String get confirmNewPassword => 'Konfirmasi kata sandi baru';

  @override
  String get confirmPassword => 'Konfirmasi kata sandi';

  @override
  String get fullName => 'Nama lengkap';

  @override
  String get forgotPassword => 'Lupa kata sandi';

  @override
  String get forgotPasswordHeadline => 'Lupa kata sandi Anda?';

  @override
  String get forgotPasswordSub => 'Masukkan email Anda, kami akan kirim tautan untuk reset.';

  @override
  String get sendResetLink => 'Kirim tautan reset';

  @override
  String get reasonOptional => 'Alasan (opsional)';

  @override
  String get reasonHint => 'Ceritakan apa yang terjadi...';

  @override
  String get backToLogin => 'Kembali ke halaman masuk';

  @override
  String get back => 'Kembali';

  @override
  String get save => 'Simpan';

  @override
  String get cancel => 'Batal';

  @override
  String get disable => 'Nonaktifkan';

  @override
  String get hiPrefix => 'Halo';

  @override
  String get noSubscription => 'Belum ada langganan';

  @override
  String get noInvoices => 'Belum ada tagihan';

  @override
  String get noNotifications => 'Belum ada notifikasi';

  @override
  String get recentInvoices => 'Tagihan Terbaru';

  @override
  String get seeAll => 'Lihat semua';

  @override
  String get notifications => 'Notifikasi';

  @override
  String get notifInvoice => 'Pengingat Tagihan';

  @override
  String get notifInvoiceSub => 'Kirim notifikasi H-3 dan jatuh tempo';

  @override
  String get notifOutage => 'Gangguan Jaringan';

  @override
  String get notifOutageSub => 'Kirim notifikasi saat ada gangguan di area Anda';

  @override
  String get notifPromo => 'Promo & Penawaran';

  @override
  String get notifPromoSub => 'Dapatkan info promo dari ISP';

  @override
  String get markAllRead => 'Tandai semua dibaca';

  @override
  String get contactUs => 'Hubungi Kami';

  @override
  String get faq => 'Pertanyaan Umum';

  @override
  String get changePassword => 'Ubah Kata Sandi';

  @override
  String get editProfile => 'Edit Profil';

  @override
  String get verifyOtp => 'Verifikasi OTP';

  @override
  String get verify2fa => 'Verifikasi 2FA';

  @override
  String get verify2faHeadline => 'Verifikasi 2 Faktor';

  @override
  String get verify => 'Verifikasi';

  @override
  String get loginWithOtp => 'Masuk dengan OTP';

  @override
  String get otpLoginHeadline => 'Masuk Tanpa Kata Sandi';

  @override
  String get otpLoginSub => 'Kami akan kirim kode 6 digit ke nomor HP Anda.';

  @override
  String get sendOtp => 'Kirim Kode OTP';

  @override
  String get otpVerifyHeadline => 'Masukkan Kode Verifikasi';

  @override
  String otpVerifySub(String phone) {
    return 'Kode telah dikirim ke $phone';
  }

  @override
  String get otpSent => 'Kode OTP telah dikirim';

  @override
  String get otpResent => 'Kode OTP dikirim ulang';

  @override
  String get resendOtp => 'Kirim ulang kode';

  @override
  String resendIn(int seconds) {
    return 'Kirim ulang dalam $seconds detik';
  }

  @override
  String get otpCode => 'Kode OTP';

  @override
  String get backupCode => 'Kode cadangan';

  @override
  String get useAuthenticator => 'Gunakan aplikasi authenticator';

  @override
  String get useBackupCode => 'Gunakan kode cadangan';

  @override
  String get twoFactorAuth => 'Autentikasi 2 Faktor';

  @override
  String get twoFaOn => 'Aktif';

  @override
  String get twoFaOff => 'Nonaktif';

  @override
  String get twoFaRequired => 'Diwajibkan organisasi';

  @override
  String get twoFaEnabled => 'Autentikasi 2 faktor berhasil diaktifkan';

  @override
  String get twoFaHeadline => 'Amankan Akun Anda';

  @override
  String get twoFaSub => 'Pindai QR ini dengan Google Authenticator atau Authy, lalu masukkan kode 6 digit.';

  @override
  String get enable2fa => 'Aktifkan 2FA';

  @override
  String get confirmEnable => 'Konfirmasi';

  @override
  String get disable2faConfirmTitle => 'Nonaktifkan 2FA?';

  @override
  String get disable2faConfirmBody => 'Akun Anda akan menjadi kurang aman. Anda dapat mengaktifkannya lagi nanti.';

  @override
  String get biometric => 'Login Biometrik';

  @override
  String get biometricSub => 'Gunakan sidik jari atau Face ID untuk login';

  @override
  String get biometricNotAvailable => 'Biometrik tidak tersedia di perangkat ini';

  @override
  String get biometricEnableReason => 'Konfirmasi untuk mengaktifkan login biometrik';

  @override
  String get passwordChanged => 'Kata sandi berhasil diubah';

  @override
  String get passwordRule => 'Minimal 8 karakter, harus ada huruf dan angka';

  @override
  String get passwordMismatch => 'Kata sandi tidak cocok';

  @override
  String get profileUpdated => 'Profil berhasil diperbarui';

  @override
  String get inviteCode => 'Kode Undangan';

  @override
  String get inviteValidateFirst => 'Validasi kode undangan terlebih dahulu';

  @override
  String get registerHeadline => 'Aktivasi Akun Anda';

  @override
  String get registerSub => 'Masukkan kode undangan dari email/WhatsApp kami';

  @override
  String get registerSuccess => 'Akun berhasil dibuat, selamat datang!';

  @override
  String get account => 'Akun';

  @override
  String get about => 'Tentang';

  @override
  String get privacyPolicy => 'Kebijakan Privasi';

  @override
  String get termsOfService => 'Syarat & Ketentuan';

  @override
  String get myInvoices => 'Tagihan Saya';

  @override
  String get mySubscriptions => 'Paket Saya';

  @override
  String get invalidEmail => 'Mohon masukkan email yang valid';

  @override
  String get passwordTooShort => 'Password minimal 8 karakter';

  @override
  String get enter2faCode => 'Masukkan 6 digit kode dari aplikasi authenticator';

  @override
  String get officeAddress => 'Alamat Kantor';

  @override
  String get serviceHours => 'Jam Layanan';

  @override
  String get myTickets => 'Tiket Saya';

  @override
  String get noTickets => 'Belum ada tiket';

  @override
  String get createFirstTicket => 'Buat tiket bantuan pertama Anda';

  @override
  String get newTicket => 'Tiket Baru';

  @override
  String get paymentInstruction => 'Instruksi Pembayaran';

  @override
  String get totalPayment => 'Total Pembayaran';

  @override
  String get choosePaymentMethod => 'Pilih Metode Pembayaran';

  @override
  String get changePasswordHeadline => 'Ubah kata sandi Anda';

  @override
  String get speedTest => 'Speed Test';

  @override
  String get pay => 'Bayar';

  @override
  String get report => 'Lapor';

  @override
  String get share => 'Bagikan';

  @override
  String get unpaidBills => 'Tagihan Belum Bayar';

  @override
  String get noBills => 'Tidak ada tagihan';

  @override
  String get activePlan => 'Paket Aktif';

  @override
  String get fromTotalSubscriptions => 'Dari total langganan';

  @override
  String get internetPackage => 'Paket Internet';

  @override
  String get quickActions => 'Aksi Cepat';

  @override
  String get subscriptionDetail => 'Detail Langganan';

  @override
  String get connectionDetails => 'Detail Koneksi';

  @override
  String get billingInfo => 'Informasi Tagihan';

  @override
  String get router => 'Router';

  @override
  String get location => 'Lokasi';

  @override
  String get notes => 'Catatan';

  @override
  String get price => 'Harga';

  @override
  String get cycle => 'Siklus Tagihan';

  @override
  String get startsAt => 'Tanggal Mulai';

  @override
  String get endsAt => 'Tanggal Berakhir';

  @override
  String get gracePeriod => 'Masa Tenggang';

  @override
  String get reportOutage => 'Laporkan Gangguan';

  @override
  String get retry => 'Coba Lagi';

  @override
  String get noPaymentUrl => 'URL pembayaran tidak tersedia';

  @override
  String get noInvoicesYet => 'Belum ada tagihan';

  @override
  String get dueOn => 'Jatuh tempo';

  @override
  String get announcements => 'Pengumuman';

  @override
  String get announcementDetail => 'Detail Pengumuman';

  @override
  String get noAnnouncements => 'Belum ada pengumuman';

  @override
  String get severity => 'Tingkat';

  @override
  String get audience => 'Audiens';

  @override
  String get details => 'Detail';

  @override
  String get darkMode => 'Mode Gelap';

  @override
  String get ticketStatusOpen => 'Terbuka';

  @override
  String get ticketStatusInProgress => 'Ditangani';

  @override
  String get ticketStatusWaitingCustomer => 'Menunggu Pelanggan';

  @override
  String get ticketStatusWaitingStaff => 'Menunggu Tim';

  @override
  String get ticketStatusResolved => 'Selesai';

  @override
  String get ticketStatusClosed => 'Ditutup';

  @override
  String get ticketStatusCancelled => 'Dibatalkan';

  @override
  String get ticketPriorityLow => 'Rendah';

  @override
  String get ticketPriorityNormal => 'Normal';

  @override
  String get ticketPriorityHigh => 'Tinggi';

  @override
  String get ticketPriorityUrgent => 'Mendesak';

  @override
  String get ticketCategoryGeneral => 'Umum';

  @override
  String get ticketCategoryBilling => 'Tagihan';

  @override
  String get ticketCategoryTechnical => 'Teknis';

  @override
  String get ticketCategoryInstallation => 'Instalasi';

  @override
  String get ticketActionCamera => 'Ambil Foto';

  @override
  String get ticketActionFile => 'Pilih File';

  @override
  String get ticketActionCameraSub => 'Kamera — perlu izin akses kamera';

  @override
  String get ticketActionFileSub => 'PDF, gambar, dokumen — dari penyimpanan perangkat';

  @override
  String ticketErrorCameraFailed(Object error) {
    return 'Gagal membuka kamera: $error';
  }

  @override
  String ticketErrorFileFailed(Object error) {
    return 'Gagal memilih file: $error';
  }

  @override
  String ticketErrorSendFailed(Object error) {
    return 'Gagal mengirim: $error';
  }

  @override
  String ticketErrorReplyFailed(Object error) {
    return 'Gagal membalas: $error';
  }

  @override
  String get ticketErrorLoadFailed => 'Gagal memuat tiket';

  @override
  String get ticketErrorSessionExpired => 'Sesi berakhir, login ulang';

  @override
  String get ticketFieldSubject => 'Subjek';

  @override
  String get ticketFieldSubjectHint => 'Ringkasan masalah';

  @override
  String get ticketFieldDescription => 'Deskripsi';

  @override
  String get ticketFieldDescriptionHint => 'Jelaskan masalah Anda...';

  @override
  String get ticketFieldReply => 'Tulis pesan...';

  @override
  String get ticketFieldAttachments => 'Lampiran';

  @override
  String get ticketFieldSubscription => 'Langganan Terkait (opsional)';

  @override
  String get ticketFieldNoSubscription => 'Tidak terkait';

  @override
  String get ticketValidationSubjectShort => 'Subjek minimal 3 karakter';

  @override
  String get ticketValidationDescriptionShort => 'Deskripsi minimal 10 karakter';

  @override
  String get ticketButtonAdd => 'Tambah';

  @override
  String get ticketButtonSend => 'Kirim Tiket';

  @override
  String get ticketButtonSending => 'Mengirim...';

  @override
  String get ticketButtonSubmitReply => 'Kirim Balasan';

  @override
  String get ticketButtonSendingReply => 'Mengirim...';

  @override
  String get ticketButtonAttach => 'Lampirkan';

  @override
  String get ticketButtonClose => 'Tutup Tiket';

  @override
  String get ticketButtonReopen => 'Buka Ulang';

  @override
  String get ticketButtonAssign => 'Tugaskan';

  @override
  String get ticketButtonEscalate => 'Eskalasi';

  @override
  String get ticketToastCreated => 'Tiket terkirim — tim kami akan menindak lanjuti';

  @override
  String get ticketToastReplySent => 'Balasan terkirim';

  @override
  String get ticketToastClosed => 'Tiket ditutup';

  @override
  String get ticketToastReopened => 'Tiket dibuka ulang';

  @override
  String get ticketQuickActionNoInternet => 'Internet Mati';

  @override
  String get ticketQuickActionNoInternetSubject => 'Internet tidak bisa diakses';

  @override
  String get ticketQuickActionNoInternetDesc => 'Koneksi internet di lokasi saya tidak dapat diakses. Mohon dicek.';

  @override
  String get ticketQuickActionSlow => 'WiFi Lemot';

  @override
  String get ticketQuickActionSlowSubject => 'WiFi lambat / sering putus';

  @override
  String get ticketQuickActionSlowDesc => 'Koneksi WiFi terasa lambat atau tidak stabil. Mohon dicek.';

  @override
  String get ticketQuickActionOther => 'Lainnya';

  @override
  String get ticketAuthorYou => 'Anda';

  @override
  String get ticketAuthorSupport => 'Dukungan';

  @override
  String get ticketAuthorCustomer => 'Pelanggan';

  @override
  String get ticketAuthorStaff => 'Staf';

  @override
  String get ticketAuthorAnonymous => 'Anonim';

  @override
  String get ticketViewSubscription => 'Lihat langganan terkait';

  @override
  String get ticketResolve => 'Selesaikan Tiket';

  @override
  String get ticketResolveHint => 'Catatan penyelesaian (opsional)';

  @override
  String get ticketResolveConfirm => 'Selesaikan';

  @override
  String get ticketResolved => 'Tiket diselesaikan';

  @override
  String get ticketClaim => 'Ambil Tiket';

  @override
  String get ticketClaimSuccess => 'Tiket berhasil diambil, sekarang di-assign ke Anda';

  @override
  String get ticketConversation => 'Percakapan';

  @override
  String get ticketNoMessages => 'Belum ada pesan';

  @override
  String get ticketNoMessagesHint => 'Kirim pesan pertama Anda untuk memulai percakapan';

  @override
  String get ticketClosedNotice => 'Tiket ini sudah ditutup. Anda tidak bisa membalas.';

  @override
  String get ticketAssignee => 'Penanggung Jawab';

  @override
  String get ticketUnassigned => 'Belum ditugaskan';

  @override
  String get ticketSatisfaction => 'Kepuasan';

  @override
  String get ticketRateHint => 'Seberapa puas Anda?';

  @override
  String get ticketCommentOptional => 'Komentar (opsional)';

  @override
  String get ticketSubmitRating => 'Kirim Penilaian';

  @override
  String get ticketAdminListTitle => 'Tiket Support';

  @override
  String get ticketAdminTabAll => 'Semua';

  @override
  String get ticketAdminTabOpen => 'Terbuka';

  @override
  String get ticketAdminTabInProgress => 'Dikerjakan';

  @override
  String get ticketAdminTabClosed => 'Selesai';

  @override
  String get ticketAdminEmpty => 'Belum ada tiket';

  @override
  String get ticketAdminFilterAll => 'Semua kategori';

  @override
  String get ticketAdminFilterOpen => 'Hanya yang terbuka';

  @override
  String get twoFaDisabledSuccess => '2FA berhasil dinonaktifkan';

  @override
  String get enterVerificationCode => 'Masukkan Kode Verifikasi';

  @override
  String get otpSentToEmail => 'Kode OTP telah dikirim ke email Anda';

  @override
  String get verificationCode => 'Kode Verifikasi';

  @override
  String get tickets => 'Tiket';

  @override
  String get ticketStatsAll => 'Semua';

  @override
  String get ticketStatsOpen => 'Buka';

  @override
  String get ticketStatsPending => 'Tertunda';

  @override
  String get ticketStatsClosed => 'Selesai';

  @override
  String get recentTickets => 'Tiket Terbaru';

  @override
  String get noAssignedTickets => 'Tidak ada tiket';

  @override
  String get assignedToYou => 'Ditugaskan ke Anda';

  @override
  String get workOrders => 'Tugas Kerja';

  @override
  String get workOrderDetail => 'Detail Tugas';

  @override
  String get workOrderClaim => 'Ambil Tugas';

  @override
  String get workOrderStart => 'Mulai Pengerjaan';

  @override
  String get workOrderComplete => 'Selesaikan';

  @override
  String get workOrderCancel => 'Batalkan';

  @override
  String get workOrderReopen => 'Buka Ulang';

  @override
  String get workOrderNotes => 'Catatan';

  @override
  String get workOrderNotesHint => 'Catatan instalasi, kendala, dll.';

  @override
  String get workOrderSelectTerminalAsset => 'Pilih terminal asset yang terpasang:';

  @override
  String get workOrderNoAssetAvailable => 'Tidak ada terminal asset tersedia. Buat asset terlebih dahulu.';

  @override
  String get workOrderStatusPending => 'Menunggu';

  @override
  String get workOrderStatusAssigned => 'Ditugaskan';

  @override
  String get workOrderStatusInProgress => 'Dikerjakan';

  @override
  String get workOrderStatusCompleted => 'Selesai';

  @override
  String get workOrderStatusCancelled => 'Dibatalkan';

  @override
  String get workOrderNoAssigned => 'Belum ada tugas';

  @override
  String get workOrderTabAll => 'Semua';

  @override
  String get workOrderTabPending => 'Menunggu';

  @override
  String get workOrderTabInProgress => 'Dikerjakan';

  @override
  String get workOrderTabCompleted => 'Selesai';

  @override
  String get workOrderConfirmed => 'Tugas selesai';

  @override
  String get workOrderClaimed => 'Tugas diambil';

  @override
  String get workOrderStarted => 'Pengerjaan dimulai';

  @override
  String get workOrderCancelled => 'Tugas dibatalkan';

  @override
  String get workOrderErrorLoad => 'Gagal memuat tugas';

  @override
  String get workOrderPackage => 'Paket';

  @override
  String get workOrderCustomer => 'Pelanggan';

  @override
  String get workOrderSchedule => 'Jadwal';

  @override
  String get workOrderRouter => 'Router';

  @override
  String get workOrderLocation => 'Lokasi';

  @override
  String get workOrderStepClaim => 'Ambil';

  @override
  String get workOrderStepStart => 'Mulai';

  @override
  String get workOrderStepComplete => 'Selesai';

  @override
  String get workOrderStepDone => '✓';

  @override
  String get workOrderPhone => 'Telpon';

  @override
  String get workOrderWhatsApp => 'WA';

  @override
  String get workOrderMaps => 'Maps';

  @override
  String get homeNoTasksToday => 'Tidak ada tugas hari ini';

  @override
  String get homeTasksToday => 'Tugas Hari Ini';

  @override
  String get homeActiveTickets => 'Tiket';

  @override
  String get homeActiveTasks => 'Tugas';

  @override
  String get homeToday => 'Hari Ini';

  @override
  String get homeTapToStart => 'Tap untuk mulai';

  @override
  String get homeTapToView => 'Tap untuk lihat';

  @override
  String get noPhoneNumber => 'Tidak ada nomor HP';
}
