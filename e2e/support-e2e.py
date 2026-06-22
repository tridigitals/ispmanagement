#!/usr/bin/env python3
"""E2E tests for Support Ticket Assignment System"""
import urllib.request
import urllib.error
import json

BASE = "http://localhost:1420"
EMAIL = "info@xtrabit.com"
PASSWORD = "sasaicoco"

def api(method, path, token=None, data=None):
    url = BASE + path
    body = json.dumps(data).encode() if data else None
    req = urllib.request.Request(url, data=body, method=method)
    req.add_header("Content-Type", "application/json")
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            content = resp.read()
            return resp.status, json.loads(content) if content else {}
    except urllib.error.HTTPError as e:
        body = e.read()
        return e.code, json.loads(body) if body else {}
    except urllib.error.URLError as e:
        return 0, {"error": str(e)}

print("=" * 50)
print("E2E: Support Ticket Assignment System")
print("=" * 50)

# 01 - Login
print("\n[01] Login")
code, body = api("POST", "/api/auth/login", data={"email": EMAIL, "password": PASSWORD})
assert code == 200, f"Login failed: {code} {body}"
token = body.get("token") or body.get("data", {}).get("token")
assert token and len(token) > 10, f"Bad token: {token[:20]}..."
print(f"    ✅ Logged in as {EMAIL}")
print(f"    Token: {token[:30]}...")

# 02 - List tickets (admin sees all)
print("\n[02] List tickets (admin)")
code, body = api("GET", "/api/support/tickets?per_page=5", token=token)
assert code == 200, f"List failed: {code} {body}"
tickets = body.get("data") or body.get("tickets") or []
print(f"    ✅ Admin sees {len(tickets)} tickets")
for t in tickets[:3]:
    print(f"       - {t['id'][:8]}... | status={t.get('status')} | assigned={t.get('assigned_to') or 'NULL'}")

# 03 - Filter unassigned
print("\n[03] Filter unassigned tickets")
code, body = api("GET", "/api/support/tickets?assigned=unassigned&per_page=10", token=token)
assert code == 200, f"Filter unassigned failed: {code} {body}"
unassigned = body.get("data") or body.get("tickets") or []
all_unassigned = all(not t.get("assigned_to") for t in unassigned)
print(f"    ✅ {len(unassigned)} unassigned tickets")
print(f"    ✅ All have no assigned_to: {all_unassigned}")
assert all_unassigned, "Some tickets have assigned_to!"

first_id = unassigned[0]["id"] if unassigned else None
if first_id:
    print(f"    First ticket: {first_id}")

# 04 - Filter assigned
print("\n[04] Filter assigned tickets")
code, body = api("GET", "/api/support/tickets?assigned=assigned&per_page=10", token=token)
assert code == 200, f"Filter assigned failed: {code} {body}"
assigned = body.get("data") or body.get("tickets") or []
all_assigned = all(t.get("assigned_to") for t in assigned)
print(f"    ✅ {len(assigned)} assigned tickets")
if assigned:
    print(f"    ✅ All have assigned_to: {all_assigned}")

# 05 - List assignees (role_level >= 25)
print("\n[05] List assignees (role_level >= 25)")
code, body = api("GET", "/api/support/assignees", token=token)
assert code == 200, f"List assignees failed: {code} {body}"
assignees = body if isinstance(body, list) else body.get("data") or []
print(f"    ✅ {len(assignees)} assignees")
for a in assignees:
    rl = a.get("role_level")
    assert rl is not None, f"{a['name']} missing role_level"
    assert rl >= 25, f"{a['name']} role_level={rl} < 25"
    assert a.get("is_active") == True, f"{a['name']} not active"
    print(f"       - {a['name']} | role_level={rl} | active={a['is_active']}")

# 06 - Assign a ticket
print("\n[06] Assign ticket flow")
if first_id and assignees:
    assignee_id = assignees[0]["user_id"]
    print(f"    Assigning ticket {first_id[:8]}... → {assignees[0]['name']}")
    code, body = api("PUT", f"/api/support/tickets/{first_id}", token=token,
                      data={"assignedTo": assignee_id})
    assert code == 200, f"Assign failed: {code} {body}"
    print(f"    ✅ PATCH succeeded")

    # Verify in assigned filter
    code, body = api("GET", "/api/support/tickets?assigned=assigned&per_page=20", token=token)
    assigned_now = body.get("data") or body.get("tickets") or []
    assigned_ids = [t["id"] for t in assigned_now]
    assert first_id in assigned_ids, f"Ticket {first_id[:8]}... NOT in assigned filter!"
    print(f"    ✅ Ticket appears in 'assigned' filter ({len(assigned_now)} total)")

    # Verify NOT in unassigned filter
    code, body = api("GET", "/api/support/tickets?assigned=unassigned&per_page=20", token=token)
    unassigned_now = body.get("data") or body.get("tickets") or []
    unassigned_ids = [t["id"] for t in unassigned_now]
    assert first_id not in unassigned_ids, f"Ticket still in unassigned filter!"
    print(f"    ✅ Ticket removed from 'unassigned' filter ({len(unassigned_now)} remaining)")
else:
    print(f"    ⏭ Skipped (no unassigned ticket or no assignees)")

# 07 - Stats
print("\n[07] Stats endpoint")
code, body = api("GET", "/api/support/tickets/stats", token=token)
assert code == 200, f"Stats failed: {code} {body}"
print(f"    ✅ Stats: {json.dumps(body)}")

print("\n" + "=" * 50)
print("ALL TESTS PASSED ✅")
print("=" * 50)
