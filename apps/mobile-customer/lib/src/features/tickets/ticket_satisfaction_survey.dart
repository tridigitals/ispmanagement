import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../services/settings_providers.dart';
import '../../services/service_providers.dart';

/// Local-only ticket satisfaction rating.
/// Stores ratings in SharedPreferences keyed by ticket ID.
class TicketSatisfactionSurvey extends ConsumerStatefulWidget {
  const TicketSatisfactionSurvey({
    required this.ticketId,
    super.key,
  });

  final String ticketId;

  @override
  ConsumerState<TicketSatisfactionSurvey> createState() =>
      _TicketSatisfactionSurveyState();
}

class _TicketSatisfactionSurveyState
    extends ConsumerState<TicketSatisfactionSurvey> {
  int _rating = 0;
  final _commentCtrl = TextEditingController();
  bool _submitted = false;
  bool _loading = true;
  late final IspThemeColors isp;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    isp = context.isp;
  }

  @override
  void initState() {
    super.initState();
    _loadExistingRating();
  }

  @override
  void dispose() {
    _commentCtrl.dispose();
    super.dispose();
  }

  Future<void> _loadExistingRating() async {
    final prefs = ref.read(sharedPreferencesProvider).valueOrNull;
    if (prefs == null) {
      setState(() => _loading = false);
      return;
    }
    final existing = prefs.getInt('ticket_rating_${widget.ticketId}');
    final existingComment =
        prefs.getString('ticket_comment_${widget.ticketId}');
    if (existing != null && existing > 0) {
      setState(() {
        _rating = existing;
        _commentCtrl.text = existingComment ?? '';
        _submitted = true;
        _loading = false;
      });
    } else {
      setState(() => _loading = false);
    }
  }

  Future<void> _submit() async {
    if (_rating == 0) return;

    // Call backend
    final svc = ref.read(ticketServiceProvider);
    final result = await svc.submitSatisfaction(
      ticketId: widget.ticketId,
      rating: _rating,
      comment:
          _commentCtrl.text.trim().isNotEmpty ? _commentCtrl.text.trim() : null,
    );

    // Cache locally too (for offline / re-entry)
    final prefs = ref.read(sharedPreferencesProvider).valueOrNull;
    if (prefs != null) {
      await prefs.setInt('ticket_rating_${widget.ticketId}', _rating);
      if (_commentCtrl.text.trim().isNotEmpty) {
        await prefs.setString(
          'ticket_comment_${widget.ticketId}',
          _commentCtrl.text.trim(),
        );
      }
    }

    if (result is Failure) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Gagal mengirim: ${(result).exception.message}'),
            backgroundColor: isp.danger,
          ),
        );
      }
      return;
    }

    setState(() => _submitted = true);

    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Terima kasih atas penilaian Anda!'),
          backgroundColor: isp.success,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {


    if (_loading) return const SizedBox.shrink();

    return Container(
      margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: isp.surfaceTertiary,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: isp.border, width: 1.5),
        boxShadow: [BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0)],
      ),
      child: _submitted ? _buildSubmitted() : _buildForm(),
    );
  }

  Widget _buildForm() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Icon(Icons.rate_review, size: 20, color: isp.accent),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                'Bagaimana pelayanan kami?',
                style: TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.w600,
                  color: isp.textPrimary,
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        // Star rating
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: List.generate(5, (i) {
            final starNum = i + 1;
            return GestureDetector(
              onTap: () => setState(() => _rating = starNum),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 4),
                child: Icon(
                  starNum <= _rating ? Icons.star : Icons.star_border,
                  size: 36,
                  color: starNum <= _rating
                      ? isp.warning
                      : isp.textMuted,
                ),
              ),
            );
          }),
        ),
        if (_rating > 0) ...[
          const SizedBox(height: 8),
          Center(
            child: Text(
              _ratingLabel(_rating),
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w500,
                color: _ratingColor(_rating),
              ),
            ),
          ),
        ],
        const SizedBox(height: 12),
        // Comment field
        TextField(
          controller: _commentCtrl,
          maxLines: 2,
          decoration: InputDecoration(
            hintText: 'Komentar (opsional)',
            hintStyle:
                TextStyle(fontSize: 13, color: isp.textMuted),
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(8),
              borderSide: BorderSide(color: isp.border),
            ),
            enabledBorder: OutlineInputBorder(
              borderRadius: BorderRadius.circular(8),
              borderSide: BorderSide(color: isp.border),
            ),
            focusedBorder: OutlineInputBorder(
              borderRadius: BorderRadius.circular(8),
              borderSide: BorderSide(color: isp.accent),
            ),
            contentPadding:
                const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            isDense: true,
          ),
          style: TextStyle(fontSize: 13, color: isp.textPrimary),
        ),
        const SizedBox(height: 12),
        SizedBox(
          width: double.infinity,
          child: ElevatedButton(
            onPressed: _rating > 0 ? _submit : null,
            style: ElevatedButton.styleFrom(
              backgroundColor: isp.accent,
              foregroundColor: Colors.white,
              padding: const EdgeInsets.symmetric(vertical: 10),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(8),
              ),
            ),
            child: const Text(
              'Kirim Penilaian',
              style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildSubmitted() {
    return Column(
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: List.generate(5, (i) {
            return Padding(
              padding: const EdgeInsets.symmetric(horizontal: 2),
              child: Icon(
                (i + 1) <= _rating ? Icons.star : Icons.star_border,
                size: 24,
                color: (i + 1) <= _rating
                    ? isp.warning
                    : isp.textMuted,
              ),
            );
          }),
        ),
        const SizedBox(height: 8),
        Text(
          'Terima kasih atas penilaian Anda!',
          style: TextStyle(
            fontSize: 13,
            fontWeight: FontWeight.w500,
            color: isp.textPrimary,
          ),
        ),
        if (_commentCtrl.text.trim().isNotEmpty) ...[
          const SizedBox(height: 4),
          Text(
            '"${_commentCtrl.text.trim()}"',
            style: TextStyle(
              fontSize: 12,
              fontStyle: FontStyle.italic,
              color: isp.textMuted,
            ),
            textAlign: TextAlign.center,
          ),
        ],
      ],
    );
  }

  String _ratingLabel(int rating) {
    switch (rating) {
      case 1:
        return 'Sangat Tidak Puas';
      case 2:
        return 'Tidak Puas';
      case 3:
        return 'Cukup';
      case 4:
        return 'Puas';
      case 5:
        return 'Sangat Puas';
      default:
        return '';
    }
  }

  Color _ratingColor(int rating) {
    if (rating <= 2) return isp.danger;
    if (rating == 3) return isp.warning;
    return isp.success;
  }
}
