import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../services/settings_providers.dart';

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
    setState(() => _submitted = true);

    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Terima kasih atas penilaian Anda!'),
          backgroundColor: Color(0xFF10B981),
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
        color: const Color(0xFFF0F9FF),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: const Color(0xFFBAE6FD)),
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
            const Icon(Icons.rate_review, size: 20, color: Color(0xFF0284C7)),
            const SizedBox(width: 8),
            const Expanded(
              child: Text(
                'Bagaimana pelayanan kami?',
                style: TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.w600,
                  color: Color(0xFF0C4A6E),
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
                      ? const Color(0xFFF59E0B)
                      : const Color(0xFFCBD5E1),
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
            hintStyle: const TextStyle(fontSize: 13, color: Color(0xFF94A3B8)),
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(8),
              borderSide: const BorderSide(color: Color(0xFFCBD5E1)),
            ),
            contentPadding:
                const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            isDense: true,
          ),
          style: const TextStyle(fontSize: 13),
        ),
        const SizedBox(height: 12),
        SizedBox(
          width: double.infinity,
          child: ElevatedButton(
            onPressed: _rating > 0 ? _submit : null,
            style: ElevatedButton.styleFrom(
              backgroundColor: const Color(0xFF0284C7),
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
                    ? const Color(0xFFF59E0B)
                    : const Color(0xFFCBD5E1),
              ),
            );
          }),
        ),
        const SizedBox(height: 8),
        Text(
          'Terima kasih atas penilaian Anda!',
          style: const TextStyle(
            fontSize: 13,
            fontWeight: FontWeight.w500,
            color: Color(0xFF0C4A6E),
          ),
        ),
        if (_commentCtrl.text.trim().isNotEmpty) ...[
          const SizedBox(height: 4),
          Text(
            '"${_commentCtrl.text.trim()}"',
            style: const TextStyle(
              fontSize: 12,
              fontStyle: FontStyle.italic,
              color: Color(0xFF64748B),
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
    if (rating <= 2) return const Color(0xFFEF4444);
    if (rating == 3) return const Color(0xFFF59E0B);
    return const Color(0xFF10B981);
  }
}
