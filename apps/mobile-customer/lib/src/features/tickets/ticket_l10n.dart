// Ticket-specific AppLocalizations helpers.
//
// These extensions map raw enum/keyword strings (returned by the model
// layers) to localized display strings via the AppLocalizations keys.
//
// Pattern: keep the model layer pure (returns enum string keys like
// 'open', 'technical', 'staff') — UI calls these extensions to get the
// human-readable label in the active locale.

import 'package:api_client/api_client.dart';

import '../../l10n/app_localizations.dart';

extension TicketStatusL10n on AppLocalizations {
  String ticketStatusLabel(TicketStatus status) {
    switch (status) {
      case TicketStatus.open:
        return ticketStatusOpen;
      case TicketStatus.inProgress:
        return ticketStatusInProgress;
      case TicketStatus.waitingCustomer:
        return ticketStatusWaitingCustomer;
      case TicketStatus.waitingStaff:
        return ticketStatusWaitingStaff;
      case TicketStatus.resolved:
        return ticketStatusResolved;
      case TicketStatus.closed:
        return ticketStatusClosed;
      case TicketStatus.cancelled:
        return ticketStatusCancelled;
    }
  }

  String ticketPriorityLabel(TicketPriority priority) {
    switch (priority) {
      case TicketPriority.low:
        return ticketPriorityLow;
      case TicketPriority.normal:
        return ticketPriorityNormal;
      case TicketPriority.high:
        return ticketPriorityHigh;
      case TicketPriority.urgent:
        return ticketPriorityUrgent;
    }
  }

  /// Map a category keyword (from API) to the localized label. Falls back
  /// to the keyword itself if unknown — caller can decide what to display.
  String ticketCategoryLabel(String? category) {
    switch (category) {
      case 'general':
        return ticketCategoryGeneral;
      case 'billing':
        return ticketCategoryBilling;
      case 'technical':
        return ticketCategoryTechnical;
      case 'installation':
        return ticketCategoryInstallation;
      default:
        return category ?? ticketCategoryGeneral;
    }
  }

  /// Map an author role/keyword to the localized label.
  /// Handles the special case where the message author is the current
  /// user (return ticketAuthorYou instead of generic "Customer").
  String ticketAuthorLabel(String? roleOrName, {required bool isCurrentUser}) {
    if (isCurrentUser) return ticketAuthorYou;
    switch (roleOrName) {
      case 'staff':
      case 'admin':
        return ticketAuthorStaff;
      case 'customer':
      case 'user':
        return ticketAuthorCustomer;
      case 'anonymous':
      case 'anonim':
        return ticketAuthorAnonymous;
      case 'support':
        return ticketAuthorSupport;
      default:
        // Treat any other value as a real display name from the API.
        // (Backend populates author_name from users.name.)
        if (roleOrName == null || roleOrName.isEmpty)
          return ticketAuthorAnonymous;
        return roleOrName;
    }
  }
}
