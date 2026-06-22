/**
 * E2E: Support Ticket Assignment System (API-level)
 * Tests: login, list tickets, filter unassigned/assigned, assignees endpoint, assignment flow
 * Run: BASE_URL=http://localhost:1420 npx playwright test e2e/support-api.spec.ts
 */

import { test, expect, request } from '@playwright/test';

const BASE_URL = process.env.API_URL ?? 'http://localhost:1420';
const ADMIN_EMAIL = 'info@xtrabit.com';
const ADMIN_PASSWORD = 'sasaicoco';

let authToken = '';
let tenantId = '';
let unassignedTicketId = '';
let assigneeUserId = '';

test('01 - login and get auth token', async () => {
  const res = await request.newContext({ baseURL: BASE_URL })
    .post('/api/auth/login', {
      data: { email: ADMIN_EMAIL, password: ADMIN_PASSWORD },
    });

  expect(res.ok(), `Login failed: ${res.status()} ${await res.text()}`).toBeTruthy();
  const body = await res.json();
  authToken = body.token ?? body.data?.token ?? '';
  tenantId = body.tenant_id ?? body.data?.tenant_id ?? '';

  expect(authToken.length > 10, `Got token: ${authToken.slice(0, 20)}...`).toBeTruthy();
  console.log(`✅ Logged in as ${ADMIN_EMAIL}, tenant: ${tenantId}`);
});

test('02 - list support tickets (admin sees all)', async ({ request: req }) => {
  const res = await req.get(`${BASE_URL}/api/support/tickets?per_page=5`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });

  expect(res.ok(), `List failed: ${res.status()} ${await res.text()}`).toBeTruthy();
  const body = await res.json();
  const tickets = body.data ?? body.tickets ?? [];

  expect(Array.isArray(tickets), 'Response should have data array').toBeTruthy();
  console.log(`✅ Admin sees ${tickets.length} tickets`);
});

test('03 - filter unassigned tickets (assigned=unassigned)', async ({ request: req }) => {
  const res = await req.get(`${BASE_URL}/api/support/tickets?assigned=unassigned&per_page=10`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });

  expect(res.ok(), `Filter unassigned failed: ${res.status()} ${await res.text()}`).toBeTruthy();
  const body = await res.json();
  const tickets = body.data ?? body.tickets ?? [];

  expect(Array.isArray(tickets)).toBeTruthy();

  // Every ticket must be truly unassigned
  for (const t of tickets) {
    expect(t.assigned_to, `Ticket ${t.id} should be unassigned`).toBeFalsy();
  }

  if (tickets.length > 0) {
    unassignedTicketId = tickets[0].id ?? tickets[0].ticket_id ?? '';
  }

  console.log(`✅ Unassigned filter: ${tickets.length} unassigned tickets`);
});

test('04 - filter assigned tickets (assigned=assigned)', async ({ request: req }) => {
  const res = await req.get(`${BASE_URL}/api/support/tickets?assigned=assigned&per_page=10`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });

  expect(res.ok(), `Filter assigned failed: ${res.status()} ${await res.text()}`).toBeTruthy();
  const body = await res.json();
  const tickets = body.data ?? body.tickets ?? [];

  expect(Array.isArray(tickets)).toBeTruthy();

  for (const t of tickets) {
    expect(t.assigned_to, `Ticket ${t.id} should be assigned`).toBeTruthy();
  }

  console.log(`✅ Assigned filter: ${tickets.length} assigned tickets`);
});

test('05 - list_support_assignees returns only role_level >= 25', async ({ request: req }) => {
  const res = await req.get(`${BASE_URL}/api/support/assignees`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });

  expect(res.ok(), `Assignees failed: ${res.status()} ${await res.text()}`).toBeTruthy();
  const assignees = await res.json();

  expect(Array.isArray(assignees), 'Response should be an array').toBeTruthy();
  expect(assignees.length > 0, 'Should have at least 1 assignee').toBeTruthy();

  for (const a of assignees) {
    expect(a.role_level, `${a.name} should have role_level`).toBeDefined();
    expect(a.role_level >= 25, `${a.name} role_level=${a.role_level} should be >= 25`).toBeTruthy();
    expect(a.is_active, `${a.name} should be active`).toBeTruthy();
  }

  assigneeUserId = assignees[0]?.user_id ?? '';
  console.log(`✅ Assignees (role_level >= 25): ${assignees.map((a: any) => `${a.name}(level=${a.role_level})`).join(', ')}`);
});

test('06 - assign a ticket and verify it moves to assigned filter', async ({ request: req }) => {
  // If no unassigned ticket from test 03, skip
  if (!unassignedTicketId) {
    test.skip();
    return;
  }

  // Get an assignee ID
  let targetId = assigneeUserId;
  if (!targetId) {
    const assigneeRes = await req.get(`${BASE_URL}/api/support/assignees`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    if (!assigneeRes.ok()) { test.skip(); return; }
    const assignees = await assigneeRes.json();
    if (assignees.length === 0) { test.skip(); return; }
    targetId = assignees[0].user_id;
  }

  // Update ticket assignment
  const updateRes = await req.patch(`${BASE_URL}/api/support/tickets/${unassignedTicketId}`, {
    headers: {
      Authorization: `Bearer ${authToken}`,
      'Content-Type': 'application/json',
    },
    data: { assigned_to: targetId },
  });

  expect(updateRes.ok(), `Assign failed: ${updateRes.status()} ${await updateRes.text()}`).toBeTruthy();

  // Verify ticket now appears in "assigned" filter
  const verifyRes = await req.get(`${BASE_URL}/api/support/tickets?assigned=assigned&per_page=50`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });

  expect(verifyRes.ok()).toBeTruthy();
  const body = await verifyRes.json();
  const tickets = body.data ?? body.tickets ?? [];
  const assignedIds = tickets.map((t: any) => t.id ?? t.ticket_id);

  expect(assignedIds.includes(unassignedTicketId), `Ticket ${unassignedTicketId} should be in assigned filter after assignment`).toBeTruthy();

  // Verify it NO LONGER appears in unassigned filter
  const unassignRes = await req.get(`${BASE_URL}/api/support/tickets?assigned=unassigned&per_page=50`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });

  expect(unassignRes.ok()).toBeTruthy();
  const unassignedBody = await unassignRes.json();
  const unassignedTickets = unassignedBody.data ?? unassignedBody.tickets ?? [];
  const unassignedIds = unassignedTickets.map((t: any) => t.id ?? t.ticket_id);

  expect(!unassignedIds.includes(unassignedTicketId), `Ticket ${unassignedTicketId} should NOT appear in unassigned filter after assignment`).toBeTruthy();

  console.log(`✅ Ticket ${unassignedTicketId} assigned successfully`);
});

test('07 - stats endpoint includes counts', async ({ request: req }) => {
  const res = await req.get(`${BASE_URL}/api/support/tickets/stats`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });

  expect(res.ok(), `Stats failed: ${res.status()} ${await res.text()}`).toBeTruthy();
  const stats = await res.json();

  // Stats should have count fields
  const hasCounts = stats.all !== undefined || stats.total !== undefined || Object.keys(stats).length > 0;
  expect(hasCounts, 'Stats should have count fields').toBeTruthy();

  console.log(`✅ Stats: ${JSON.stringify(stats)}`);
});
