import type {
  CreateBackofficeInstallationOrderRequest,
  BackofficeOrderCustomerMode,
  BackofficeOrderLocationMode,
} from '$lib/api/types';

export type OrderCustomerForm = {
  name: string;
  email: string;
  phone: string;
  notes: string;
  is_active: boolean;
};

export type OrderLocationForm = {
  label: string;
  address_line1: string;
  address_line2: string;
  city: string;
  state: string;
  postal_code: string;
  country: string;
  latitude: string;
  longitude: string;
  notes: string;
};

export type OrderWizardDraft = {
  customerMode: BackofficeOrderCustomerMode;
  existingCustomerId: string;
  customer: OrderCustomerForm;
  locationMode: BackofficeOrderLocationMode;
  existingLocationId: string;
  location: OrderLocationForm;
  packageId: string;
  billingCycle: 'monthly' | 'yearly';
  notes: string;
  requestedInstallationDate: string;
};

function trimmedOrNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function numberOrNull(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : null;
}

export function inferInitialCustomerMode(prefilledCustomerId?: string | null): BackofficeOrderCustomerMode {
  return prefilledCustomerId?.trim() ? 'existing' : 'new';
}

export function buildBackofficeInstallationOrderPayload(
  draft: OrderWizardDraft,
): CreateBackofficeInstallationOrderRequest {
  if (!draft.packageId.trim()) {
    throw new Error('Package is required');
  }

  if (draft.customerMode === 'existing' && !draft.existingCustomerId.trim()) {
    throw new Error('Existing customer is required');
  }

  if (draft.customerMode === 'new' && !draft.customer.name.trim()) {
    throw new Error('Customer name is required');
  }

  if (
    draft.customerMode === 'new' &&
    !draft.customer.email.trim() &&
    !draft.customer.phone.trim()
  ) {
    throw new Error('Customer email or phone is required');
  }

  if (draft.locationMode === 'existing' && !draft.existingLocationId.trim()) {
    throw new Error('Existing location is required');
  }

  if (draft.locationMode === 'new') {
    if (!draft.location.label.trim()) {
      throw new Error('Location label is required');
    }
    if (!draft.location.address_line1.trim()) {
      throw new Error('Location address is required');
    }
  }

  return {
    customer_mode: draft.customerMode,
    customer_id: draft.customerMode === 'existing' ? draft.existingCustomerId.trim() : null,
    customer:
      draft.customerMode === 'new'
        ? {
            name: draft.customer.name.trim(),
            email: trimmedOrNull(draft.customer.email),
            phone: trimmedOrNull(draft.customer.phone),
            notes: trimmedOrNull(draft.customer.notes),
            is_active: draft.customer.is_active,
          }
        : null,
    location_mode: draft.locationMode,
    location_id: draft.locationMode === 'existing' ? draft.existingLocationId.trim() : null,
    location:
      draft.locationMode === 'new'
        ? {
            label: draft.location.label.trim(),
            address_line1: draft.location.address_line1.trim(),
            address_line2: trimmedOrNull(draft.location.address_line2),
            city: trimmedOrNull(draft.location.city),
            state: trimmedOrNull(draft.location.state),
            postal_code: trimmedOrNull(draft.location.postal_code),
            country: trimmedOrNull(draft.location.country),
            latitude: numberOrNull(draft.location.latitude),
            longitude: numberOrNull(draft.location.longitude),
            notes: trimmedOrNull(draft.location.notes),
          }
        : null,
    package_id: draft.packageId.trim(),
    billing_cycle: draft.billingCycle,
    notes: trimmedOrNull(draft.notes),
    requested_installation_date: trimmedOrNull(draft.requestedInstallationDate),
  };
}
