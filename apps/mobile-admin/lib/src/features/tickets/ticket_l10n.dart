// Ticket-specific AppLocalizations helpers for mobile-admin.
//
// Same pattern as the customer app — model layer returns raw enum/
// keyword strings, UI uses these extensions to get localized display.

import '../../l10n/app_localizations.dart';
import 'package:api_client/api_client.dart';

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
        if (roleOrName == null || roleOrName.isEmpty) return ticketAuthorAnonymous;
        return roleOrName;
    }
  }
}
