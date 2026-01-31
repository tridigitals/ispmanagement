<script lang="ts">
    import { isSuperAdmin } from "$lib/stores/auth";
    import { user as currentUser } from "$lib/stores/auth";
    import { api } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { fly } from "svelte/transition";
    import Icon from "$lib/components/Icon.svelte";
    import Table from "$lib/components/Table.svelte";
    import TableToolbar from "$lib/components/TableToolbar.svelte";
    import StatsCard from "$lib/components/StatsCard.svelte";
    import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
    import Modal from "$lib/components/Modal.svelte";
    import Pagination from "$lib/components/Pagination.svelte";
    import { toast } from "$lib/stores/toast";
    import type { User } from "$lib/api/client";

    const columns = [
        { key: "user", label: "User" },
        { key: "email", label: "Email" },
        { key: "role", label: "Role" },
        { key: "tenant", label: "Tenant" },
        { key: "status", label: "Status" },
        { key: "joined", label: "Joined" },
        { key: "actions", label: "", align: "right" as const },
    ];

    let allUsers = $state<User[]>([]);
    let totalUsers = $state(0);
    let loading = $state(true);
    let error = $state("");

    let tenantNameById = $state<Record<string, string>>({});
    let tenantNameBySlug = $state<Record<string, string>>({});

    let searchQuery = $state("");
    let statusFilter = $state<"all" | "active" | "inactive">("all");
    let roleFilter = $state<"all" | "superadmin" | "admin" | "user">("all");

    let isMobile = $state(false);
    let viewMode = $state<"table" | "cards">("table");

    let cardPage = $state(0);
    let cardPageSize = $state(10);

    async function loadData() {
        loading = true;
        error = "";

        try {
            const [usersRes, tenantsRes] = await Promise.all([
                api.users.list(1, 200),
                api.superadmin.listTenants().catch(() => null),
            ]);

            allUsers = usersRes.data || [];
            totalUsers = usersRes.total ?? allUsers.length;

            const tenants: any[] = (tenantsRes as any)?.data || [];
            const byId: Record<string, string> = {};
            const bySlug: Record<string, string> = {};
            for (const t of tenants) {
                if (t?.id && t?.name) byId[String(t.id)] = String(t.name);
                if (t?.slug && t?.name) bySlug[String(t.slug)] = String(t.name);
            }
            tenantNameById = byId;
            tenantNameBySlug = bySlug;
        } catch (err: any) {
            console.error("Failed to load users:", err);
            error = err?.message || String(err);
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        let cleanup: (() => void) | undefined;

        if (!$isSuperAdmin) {
            goto("/dashboard");
            return cleanup;
        }

        if (typeof window !== "undefined") {
            const mq = window.matchMedia("(max-width: 720px)");
            const sync = () => (isMobile = mq.matches);
            sync();

            try {
                mq.addEventListener("change", sync);
                cleanup = () => mq.removeEventListener("change", sync);
            } catch {
                // Safari/older WebView fallback
                // @ts-ignore
                mq.addListener?.(sync);
                // @ts-ignore
                cleanup = () => mq.removeListener?.(sync);
            }
        }

        void loadData();
        return cleanup;
    });

    function getRoleKey(u: User) {
        if ((u as any).is_super_admin) return "superadmin";
        const tenantRole = (u as any).tenant_role;
        if (tenantRole) return String(tenantRole).toLowerCase();
        return String((u as any).role || "user").toLowerCase();
    }

    function getTenantName(u: any) {
        const id = u?.tenant_id ? String(u.tenant_id) : "";
        const slug = u?.tenant_slug ? String(u.tenant_slug) : "";
        return (
            (id && tenantNameById[id]) ||
            (slug && tenantNameBySlug[slug]) ||
            ""
        );
    }

    let stats = $derived({
        total: allUsers.length,
        active: allUsers.filter((u: any) => u.is_active).length,
        inactive: allUsers.filter((u: any) => !u.is_active).length,
        superadmins: allUsers.filter((u: any) => u.is_super_admin).length,
    });

    let filteredUsers = $derived(
        allUsers.filter((u: any) => {
            const q = searchQuery.trim().toLowerCase();
            const matchesSearch =
                !q ||
                String(u.name || "")
                    .toLowerCase()
                    .includes(q) ||
                String(u.email || "")
                    .toLowerCase()
                    .includes(q) ||
                String(
                    getTenantName(u) || u.tenant_slug || u.tenant_id || "",
                )
                    .toLowerCase()
                    .includes(q);

            const matchesStatus =
                statusFilter === "all" ||
                (statusFilter === "active" ? u.is_active : !u.is_active);

            const roleKey = getRoleKey(u);
            const matchesRole =
                roleFilter === "all" || roleKey === roleFilter;

            return matchesSearch && matchesStatus && matchesRole;
        }),
    );

    let filterKey = $derived(
        `${searchQuery.trim().toLowerCase()}|${statusFilter}|${roleFilter}`,
    );

    $effect(() => {
        filterKey;
        cardPage = 0;
    });

    $effect(() => {
        if (isMobile) viewMode = "cards";
    });

    let pagedUsers = $derived.by((): User[] => {
        const start = cardPage * cardPageSize;
        const end = start + cardPageSize;
        return filteredUsers.slice(start, end) as User[];
    });

    let showResetConfirm = $state(false);
    let confirmLoading = $state(false);
    let userPending2FAReset = $state<User | null>(null);

    function confirmReset2FA(u: User) {
        userPending2FAReset = u;
        showResetConfirm = true;
    }

    async function reset2FA() {
        const u = userPending2FAReset;
        if (!u) return;

        confirmLoading = true;
        try {
            await api.auth.resetUser2FA(u.id);
            // Update local state
            allUsers = allUsers.map((user) =>
                user.id === u.id
                    ? ({ ...user, two_factor_enabled: false } as any)
                    : user,
            );
            toast.success("Two-factor authentication has been reset");
            showResetConfirm = false;
        } catch (err: any) {
            toast.error("Failed to reset 2FA: " + (err?.message || err));
        } finally {
            confirmLoading = false;
            userPending2FAReset = null;
        }
    }

    let showStatusConfirm = $state(false);
    let statusConfirmLoading = $state(false);
    let userPendingStatus = $state<User | null>(null);
    let pendingIsActive = $state<boolean>(false);

    let statusConfirmTitle = $derived(
        pendingIsActive ? "Activate User" : "Deactivate User",
    );

    let statusConfirmMessage = $derived.by(() => {
        const u = userPendingStatus;
        const name = u?.name || "this user";
        if (pendingIsActive) {
            return `Activate ${name}? They will be able to login again. Type ACTIVATE to confirm.`;
        }
        return `Deactivate ${name}? They will not be able to login. Type DEACTIVATE to confirm.`;
    });

    let statusConfirmKeyword = $derived(
        pendingIsActive ? "ACTIVATE" : "DEACTIVATE",
    );

    let statusConfirmType = $derived<"danger" | "warning" | "info">(
        pendingIsActive ? "info" : "danger",
    );

    function confirmToggleActive(u: User) {
        if ((u as any).is_super_admin) {
            toast.error("Super Admin accounts cannot be deactivated here");
            return;
        }
        if (u.id === $currentUser?.id) {
            toast.error("You cannot deactivate your own account");
            return;
        }
        userPendingStatus = u;
        pendingIsActive = !Boolean((u as any).is_active);
        showStatusConfirm = true;
    }

    async function toggleActive() {
        const u = userPendingStatus;
        if (!u) return;

        statusConfirmLoading = true;
        try {
            await api.users.update(u.id, { isActive: pendingIsActive });
            allUsers = allUsers.map((x: any) =>
                x.id === u.id ? { ...x, is_active: pendingIsActive } : x,
            );
            toast.success(
                pendingIsActive ? "User activated" : "User deactivated",
            );
            showStatusConfirm = false;
        } catch (e: any) {
            toast.error(
                "Failed to update user status: " + (e?.message || e),
            );
        } finally {
            statusConfirmLoading = false;
            userPendingStatus = null;
        }
    }

    let showDetailsModal = $state(false);
    let detailsUser = $state<User | null>(null);

    function openDetails(u: User) {
        detailsUser = u;
        showDetailsModal = true;
    }

    function formatDateMaybe(value: any) {
        if (!value) return "-";
        const d = new Date(value);
        if (Number.isNaN(d.getTime())) return "-";
        return d.toLocaleString();
    }

    const getInitials = (name: string) => name.substring(0, 2).toUpperCase();
</script>

<div class="superadmin-content fade-in">
    <div class="stats-row" aria-label="User stats">
        <button
            class="stat-btn"
            class:active={statusFilter === "all"}
            onclick={() => {
                statusFilter = "all";
                roleFilter = "all";
            }}
            aria-label="Show all users"
            title="Show all users"
            type="button"
        >
            <StatsCard
                title="All Users"
                value={stats.total}
                icon="users"
                color="primary"
            />
        </button>
        <button
            class="stat-btn"
            class:active={statusFilter === "active"}
            onclick={() => (statusFilter = "active")}
            aria-label="Show active users"
            title="Show active users"
            type="button"
        >
            <StatsCard
                title="Active Users"
                value={stats.active}
                icon="check-circle"
                color="success"
            />
        </button>
        <button
            class="stat-btn"
            class:active={statusFilter === "inactive"}
            onclick={() => (statusFilter = "inactive")}
            aria-label="Show inactive users"
            title="Show inactive users"
            type="button"
        >
            <StatsCard
                title="Inactive Users"
                value={stats.inactive}
                icon="slash"
                color="warning"
            />
        </button>
        <button
            class="stat-btn"
            class:active={roleFilter === "superadmin"}
            onclick={() => {
                roleFilter = "superadmin";
                statusFilter = "all";
            }}
            aria-label="Show super admins"
            title="Show super admins"
            type="button"
        >
            <StatsCard
                title="Super Admins"
                value={stats.superadmins}
                icon="server"
                color="danger"
            />
        </button>
    </div>

    <div class="glass-card" in:fly={{ y: 20, delay: 80 }}>
        <div class="card-header glass">
            <div>
                <h3>Users</h3>
                <span class="muted">Manage global users and access</span>
            </div>
            <span class="count-badge">{totalUsers || stats.total} users</span>
        </div>

        <div class="toolbar-wrapper">
            <TableToolbar bind:searchQuery placeholder="Search users...">
                {#snippet filters()}
                    <div class="filters-row">
                        <div class="role-filter">
                            <button
                                type="button"
                                class="filter-chip"
                                class:active={roleFilter === "all"}
                                onclick={() => (roleFilter = "all")}
                            >
                                All Roles
                            </button>
                            <button
                                type="button"
                                class="filter-chip"
                                class:active={roleFilter === "admin"}
                                onclick={() => (roleFilter = "admin")}
                            >
                                Admin
                            </button>
                            <button
                                type="button"
                                class="filter-chip"
                                class:active={roleFilter === "user"}
                                onclick={() => (roleFilter = "user")}
                            >
                                User
                            </button>
                            <button
                                type="button"
                                class="filter-chip"
                                class:active={roleFilter === "superadmin"}
                                onclick={() => (roleFilter = "superadmin")}
                            >
                                Super Admin
                            </button>
                        </div>

                        <div class="status-filter">
                            <button
                                type="button"
                                class="filter-chip"
                                class:active={statusFilter === "all"}
                                onclick={() => (statusFilter = "all")}
                            >
                                All
                            </button>
                            <button
                                type="button"
                                class="filter-chip"
                                class:active={statusFilter === "active"}
                                onclick={() => (statusFilter = "active")}
                            >
                                Active
                            </button>
                            <button
                                type="button"
                                class="filter-chip"
                                class:active={statusFilter === "inactive"}
                                onclick={() => (statusFilter = "inactive")}
                            >
                                Inactive
                            </button>
                        </div>
                    </div>
                {/snippet}

                {#snippet actions()}
                    {#if !isMobile}
                        <div class="view-toggle" aria-label="View mode">
                            <button
                                type="button"
                                class="view-btn"
                                class:active={viewMode === "table"}
                                onclick={() => (viewMode = "table")}
                                title="Table view"
                                aria-label="Table view"
                            >
                                <Icon name="list" size={18} />
                            </button>
                            <button
                                type="button"
                                class="view-btn"
                                class:active={viewMode === "cards"}
                                onclick={() => (viewMode = "cards")}
                                title="Card view"
                                aria-label="Card view"
                            >
                                <Icon name="grid" size={18} />
                            </button>
                        </div>
                    {/if}
                {/snippet}
            </TableToolbar>
        </div>

        {#if error}
            <div class="error-state">
                <Icon name="alert-circle" size={48} color="#ef4444" />
                <p>{error}</p>
            </div>
        {:else if viewMode === "cards" || isMobile}
            <div class="cards-wrapper">
                {#if filteredUsers.length === 0}
                    <div class="empty-state-container">
                        <div class="empty-icon">
                            <Icon name="users" size={64} />
                        </div>
                        <h3>No users found</h3>
                        <p>Try adjusting your search or filters.</p>
                    </div>
                {:else}
                    <div class="user-cards" aria-label="Users list">
                        {#each pagedUsers as u (u.id)}
                            <div class="user-card">
                                <div class="card-top">
                                    <div class="user-info">
                                        <div class="avatar">
                                            {getInitials(u.name)}
                                        </div>
                                        <div class="user-meta">
                                            <div class="user-name">{u.name}</div>
                                            <div class="user-email text-muted">
                                                {u.email}
                                            </div>
                                        </div>
                                    </div>

                                    <div class="actions">
                                        <button
                                            class="btn-icon"
                                            onclick={() => openDetails(u)}
                                            title="View details"
                                            aria-label="View details"
                                            type="button"
                                        >
                                            <Icon name="eye" size={16} />
                                        </button>

                                        {#if (u as any).two_factor_enabled}
                                            <button
                                                class="btn-icon warning"
                                                onclick={() => confirmReset2FA(u)}
                                                title="Reset 2FA"
                                                aria-label="Reset 2FA"
                                                type="button"
                                            >
                                                <Icon name="shield-off" size={16} />
                                            </button>
                                        {/if}

                                        <button
                                            class="btn-icon {u.is_active ? 'danger' : 'success'}"
                                            onclick={() => confirmToggleActive(u)}
                                            title={u.is_active
                                                ? "Deactivate user"
                                                : "Activate user"}
                                            aria-label={u.is_active
                                                ? "Deactivate user"
                                                : "Activate user"}
                                            disabled={u.is_super_admin || u.id === $currentUser?.id}
                                            type="button"
                                        >
                                            <Icon
                                                name={u.is_active
                                                    ? "ban"
                                                    : "check-circle"}
                                                size={16}
                                            />
                                        </button>
                                    </div>
                                </div>

                                <div class="card-bottom">
                                    <div class="meta-grid">
                                        <div class="meta-item">
                                            <span class="meta-label">Role</span>
                                            <span class="meta-value">
                                                {#if u.is_super_admin}
                                                    <span class="role-pill superadmin">
                                                        Super Admin
                                                    </span>
                                                {:else if (u as any).tenant_role}
                                                    <span
                                                        class="role-pill {(u as any).tenant_role.toLowerCase()}"
                                                        >{(u as any).tenant_role}</span
                                                    >
                                                {:else}
                                                    <span class="role-pill {u.role}"
                                                        >{u.role}</span
                                                    >
                                                {/if}
                                            </span>
                                        </div>

                                        <div class="meta-item">
                                            <span class="meta-label">Tenant</span>
                                            <span class="meta-value">
                                                {#if getTenantName(u as any)}
                                                    {getTenantName(u as any)}
                                                {:else if (u as any).tenant_slug}
                                                    {(u as any).tenant_slug}
                                                {:else}
                                                    -
                                                {/if}
                                            </span>
                                        </div>

                                        <div class="meta-item">
                                            <span class="meta-label">Status</span>
                                            <span class="meta-value">
                                                {#if u.is_active}
                                                    <span class="status-pill active">
                                                        <span class="dot"></span> Active
                                                    </span>
                                                {:else}
                                                    <span class="status-pill inactive">
                                                        <span class="dot"></span> Inactive
                                                    </span>
                                                {/if}
                                            </span>
                                        </div>

                                        <div class="meta-item">
                                            <span class="meta-label">Joined</span>
                                            <span class="meta-value text-muted">
                                                {new Date(u.created_at).toLocaleDateString()}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        {/each}
                    </div>

                    <div class="cards-pagination">
                        <Pagination
                            count={filteredUsers.length}
                            page={cardPage}
                            pageSize={cardPageSize}
                            onchange={(p: number) => (cardPage = p)}
                            onpageSizeChange={(s: number) => (cardPageSize = s)}
                        />
                    </div>
                {/if}
            </div>
        {:else}
            <div class="table-wrapper">
                <Table
                    pagination={true}
                    {columns}
                    data={filteredUsers}
                    {loading}
                    emptyText="No users found"
                >
                    {#snippet empty()}
                        <div class="empty-state-container">
                            <div class="empty-icon">
                                <Icon name="users" size={64} />
                            </div>
                            <h3>No users found</h3>
                            <p>Try adjusting your search or filters.</p>
                        </div>
                    {/snippet}

                    {#snippet cell({ item, key })}
                        {#if key === "user"}
                            <div class="user-info">
                                <div class="avatar">
                                    {getInitials(item.name)}
                                </div>
                                <div>
                                    <div class="user-name">{item.name}</div>
                                </div>
                            </div>
                        {:else if key === "email"}
                            {item.email}
                        {:else if key === "role"}
                            {#if item.is_super_admin}
                                <span class="role-pill superadmin">
                                    Super Admin
                                </span>
                            {:else if item.tenant_role}
                                <span
                                    class="role-pill {item.tenant_role.toLowerCase()}"
                                    >{item.tenant_role}</span
                                >
                                {#if item.role !== "user" && item.role !== "admin" && item.role !== item.tenant_role.toLowerCase()}
                                    <span class="text-xs text-muted"
                                        >({item.role})</span
                                    >
                                {/if}
                            {:else}
                                <span class="role-pill {item.role}"
                                    >{item.role}</span
                                >
                            {/if}
                        {:else if key === "tenant"}
                            {#if getTenantName(item)}
                                <div class="tenant-cell">
                                    <div class="tenant-name">
                                        {getTenantName(item)}
                                    </div>
                                    {#if item.tenant_slug}
                                        <div class="tenant-meta text-mono">
                                            {item.tenant_slug}
                                        </div>
                                    {/if}
                                </div>
                            {:else if item.tenant_slug}
                                <div class="tenant-cell">
                                    <div class="tenant-name">
                                        {item.tenant_slug}
                                    </div>
                                </div>
                            {:else}
                                <span class="text-muted">-</span>
                            {/if}
                        {:else if key === "status"}
                            {#if item.is_active}
                                <span class="status-pill active">
                                    <span class="dot"></span> Active
                                </span>
                            {:else}
                                <span class="status-pill inactive">
                                    <span class="dot"></span> Inactive
                                </span>
                            {/if}
                        {:else if key === "joined"}
                            <span class="text-muted">
                                {new Date(item.created_at).toLocaleDateString()}
                            </span>
                        {:else if key === "actions"}
                            <div class="actions">
                                <button
                                    class="btn-icon"
                                    onclick={() => openDetails(item)}
                                    title="View details"
                                    aria-label="View details"
                                    type="button"
                                >
                                    <Icon name="eye" size={16} />
                                </button>

                                {#if item.two_factor_enabled}
                                    <button
                                        class="btn-icon warning"
                                        onclick={() => confirmReset2FA(item)}
                                        title="Reset 2FA"
                                        aria-label="Reset 2FA"
                                        type="button"
                                    >
                                        <Icon name="shield-off" size={16} />
                                    </button>
                                {/if}

                                <button
                                    class="btn-icon {item.is_active ? 'danger' : 'success'}"
                                    onclick={() => confirmToggleActive(item)}
                                    title={item.is_active
                                        ? "Deactivate user"
                                        : "Activate user"}
                                    aria-label={item.is_active
                                        ? "Deactivate user"
                                        : "Activate user"}
                                    disabled={item.is_super_admin || item.id === $currentUser?.id}
                                    type="button"
                                >
                                    <Icon
                                        name={item.is_active
                                            ? "ban"
                                            : "check-circle"}
                                        size={16}
                                    />
                                </button>
                            </div>
                        {:else}
                            {item[key]}
                        {/if}
                    {/snippet}
                </Table>
            </div>
        {/if}
    </div>
</div>

<ConfirmDialog
    bind:show={showResetConfirm}
    title="Reset Two-Factor Authentication"
    message="Reset 2FA for this user? They will be able to login without a secondary code. Type RESET to confirm."
    confirmText="Reset 2FA"
    confirmationKeyword="RESET"
    type="warning"
    loading={confirmLoading}
    onconfirm={reset2FA}
/>

<ConfirmDialog
    bind:show={showStatusConfirm}
    title={statusConfirmTitle}
    message={statusConfirmMessage}
    confirmText={pendingIsActive ? "Activate" : "Deactivate"}
    confirmationKeyword={statusConfirmKeyword}
    type={statusConfirmType}
    loading={statusConfirmLoading}
    onconfirm={toggleActive}
/>

<Modal
    bind:show={showDetailsModal}
    title={detailsUser ? `User Details — ${detailsUser.name}` : "User Details"}
    width="640px"
    onclose={() => {
        showDetailsModal = false;
        detailsUser = null;
    }}
>
    {#if detailsUser}
        <div class="details-grid">
            <div class="detail-card">
                <div class="detail-title">Account</div>
                <div class="detail-row">
                    <span class="detail-key">Name</span>
                    <span class="detail-val">{detailsUser.name}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">Email</span>
                    <span class="detail-val">{detailsUser.email}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">Role</span>
                    <span class="detail-val">
                        {#if detailsUser.is_super_admin}
                            <span class="role-pill superadmin"
                                >Super Admin</span
                            >
                        {:else}
                            <span class="role-pill {detailsUser.role}"
                                >{detailsUser.role}</span
                            >
                        {/if}
                    </span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">Status</span>
                    <span class="detail-val">
                        {#if detailsUser.is_active}
                            <span class="status-pill active">
                                <span class="dot"></span> Active
                            </span>
                        {:else}
                            <span class="status-pill inactive">
                                <span class="dot"></span> Inactive
                            </span>
                        {/if}
                    </span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">Created</span>
                    <span class="detail-val"
                        >{formatDateMaybe(detailsUser.created_at)}</span
                    >
                </div>
                <div class="detail-row">
                    <span class="detail-key">Last Login</span>
                    <span class="detail-val">
                        {formatDateMaybe(
                            (detailsUser as any).last_login_at ||
                                (detailsUser as any).last_login ||
                                (detailsUser as any).last_login_date,
                        )}
                    </span>
                </div>
            </div>

            <div class="detail-card">
                <div class="detail-title">Tenant</div>
                <div class="detail-row">
                    <span class="detail-key">Tenant</span>
                    <span class="detail-val">
                        {#if getTenantName(detailsUser as any)}
                            {getTenantName(detailsUser as any)}
                        {:else}
                            -
                        {/if}
                    </span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">Slug</span>
                    <span class="detail-val text-mono">
                        {(detailsUser as any).tenant_slug || "-"}
                    </span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">Tenant Role</span>
                    <span class="detail-val">
                        {(detailsUser as any).tenant_role || "-"}
                    </span>
                </div>
            </div>

            <div class="detail-card">
                <div class="detail-title">Security</div>
                <div class="detail-row">
                    <span class="detail-key">2FA Enabled</span>
                    <span class="detail-val">
                        {(detailsUser as any).two_factor_enabled
                            ? "Yes"
                            : "No"}
                    </span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">Preferred 2FA</span>
                    <span class="detail-val">
                        {(detailsUser as any).preferred_2fa_method || "-"}
                    </span>
                </div>
            </div>
        </div>
    {/if}
</Modal>

<style>
    .superadmin-content {
        padding: clamp(16px, 3vw, 32px);
        max-width: 1400px;
        margin: 0 auto;
        color: var(--text-primary);
        --glass: rgba(255, 255, 255, 0.04);
        --glass-border: rgba(255, 255, 255, 0.08);
    }

    .stats-row {
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
        gap: 1rem;
        margin-bottom: 1.25rem;
    }

    .stat-btn {
        border: none;
        padding: 0;
        background: transparent;
        cursor: pointer;
        text-align: left;
        border-radius: 18px;
        transition: transform 0.15s ease;
    }

    .stat-btn:hover {
        transform: translateY(-1px);
    }

    .stat-btn.active :global(.stats-card) {
        border-color: rgba(99, 102, 241, 0.35);
        box-shadow: 0 0 0 1px rgba(99, 102, 241, 0.25);
    }

    .glass-card {
        background: var(--glass);
        border: 1px solid var(--glass-border);
        border-radius: var(--radius-lg);
        overflow: hidden;
        box-shadow: 0 18px 45px rgba(0, 0, 0, 0.35);
        backdrop-filter: blur(12px);
    }

    :global([data-theme="light"]) .glass-card {
        background: rgba(255, 255, 255, 0.75);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 12px 28px rgba(0, 0, 0, 0.06),
            0 0 0 1px rgba(255, 255, 255, 0.85);
    }

    .card-header {
        padding: 1.25rem 1.25rem 1rem 1.25rem;
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 1rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    }

    :global([data-theme="light"]) .card-header {
        border-bottom-color: rgba(0, 0, 0, 0.06);
    }

    .card-header h3 {
        margin: 0;
        font-size: 1.1rem;
        font-weight: 800;
        color: var(--text-primary);
        letter-spacing: -0.01em;
    }

    .muted {
        display: block;
        margin-top: 0.25rem;
        color: var(--text-secondary);
        font-size: 0.92rem;
    }

    .count-badge {
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.08);
        color: var(--text-primary);
        padding: 0.35rem 0.75rem;
        border-radius: 999px;
        font-size: 0.85rem;
        font-weight: 650;
        white-space: nowrap;
        align-self: flex-start;
    }

    :global([data-theme="light"]) .count-badge {
        background: rgba(0, 0, 0, 0.03);
        border-color: rgba(0, 0, 0, 0.06);
    }

    .toolbar-wrapper {
        padding: 1rem 1.25rem 0.25rem 1.25rem;
    }

    .filters-row {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
        align-items: center;
        width: 100%;
    }

    .status-filter,
    .role-filter {
        display: inline-flex;
        align-items: center;
        gap: 0.35rem;
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 12px;
        padding: 0.35rem;
        flex-wrap: wrap;
    }

    :global([data-theme="light"]) .status-filter,
    :global([data-theme="light"]) .role-filter {
        background: rgba(0, 0, 0, 0.02);
        border-color: rgba(0, 0, 0, 0.06);
    }

    .filter-chip {
        border: none;
        background: transparent;
        color: var(--text-secondary);
        padding: 0.45rem 0.75rem;
        border-radius: 10px;
        cursor: pointer;
        font-weight: 650;
        font-size: 0.85rem;
        transition: all 0.2s;
        white-space: nowrap;
    }

    .filter-chip:hover {
        color: var(--text-primary);
        background: rgba(255, 255, 255, 0.05);
    }

    :global([data-theme="light"]) .filter-chip:hover {
        background: rgba(0, 0, 0, 0.04);
    }

    .filter-chip.active {
        background: rgba(99, 102, 241, 0.18);
        border: 1px solid rgba(99, 102, 241, 0.25);
        color: var(--text-primary);
    }

    .table-wrapper {
        padding: 0 1.25rem 1rem 1.25rem;
    }

    .view-toggle {
        display: inline-flex;
        align-items: center;
        gap: 0.25rem;
        padding: 0.25rem;
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        background: rgba(255, 255, 255, 0.03);
    }

    :global([data-theme="light"]) .view-toggle {
        border-color: rgba(0, 0, 0, 0.06);
        background: rgba(0, 0, 0, 0.02);
    }

    .view-btn {
        width: 38px;
        height: 38px;
        border-radius: 10px;
        border: none;
        background: transparent;
        color: var(--text-secondary);
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
        padding: 0;
    }

    .view-btn:hover {
        background: rgba(255, 255, 255, 0.05);
        color: var(--text-primary);
    }

    :global([data-theme="light"]) .view-btn:hover {
        background: rgba(0, 0, 0, 0.04);
    }

    .view-btn.active {
        background: rgba(99, 102, 241, 0.18);
        border: 1px solid rgba(99, 102, 241, 0.25);
        color: var(--text-primary);
    }

    .user-info {
        display: flex;
        align-items: center;
        gap: 0.85rem;
    }

    .avatar {
        width: 42px;
        height: 42px;
        background: linear-gradient(135deg, #475569, #334155);
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 700;
        color: white;
        font-size: 0.9rem;
    }

    .user-name {
        font-weight: 600;
        color: var(--text-primary);
    }

    .text-xs {
        font-size: 0.78rem;
    }

    .tenant-cell {
        display: flex;
        flex-direction: column;
        gap: 0.15rem;
        min-width: 0;
    }

    .tenant-name {
        font-weight: 650;
        color: var(--text-primary);
        line-height: 1.15;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .tenant-meta {
        opacity: 0.85;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .role-pill {
        padding: 0.3rem 0.8rem;
        border-radius: 20px;
        font-size: 0.8rem;
        font-weight: 600;
        text-transform: capitalize;
    }

    .role-pill.admin {
        background: rgba(99, 102, 241, 0.2);
        color: #818cf8;
    }

    .role-pill.superadmin {
        background: rgba(139, 92, 246, 0.2);
        color: #a78bfa;
        border: 1px solid rgba(139, 92, 246, 0.3);
    }

    .role-pill.user {
        background: rgba(16, 185, 129, 0.2);
        color: #34d399;
    }

    .status-pill {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.35rem 0.8rem;
        border-radius: 20px;
        font-size: 0.8rem;
        font-weight: 600;
    }

    .status-pill.active {
        background: rgba(16, 185, 129, 0.15);
        color: var(--color-success);
        border: 1px solid rgba(16, 185, 129, 0.2);
    }

    .status-pill.inactive {
        background: rgba(239, 68, 68, 0.15);
        color: var(--color-danger);
        border: 1px solid rgba(239, 68, 68, 0.2);
    }

    .dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: currentColor;
    }

    .text-muted {
        color: var(--text-muted);
    }
    .text-mono {
        font-family: monospace;
        font-size: 0.85rem;
        color: var(--text-secondary);
    }

    .btn-icon:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .btn-icon {
        width: 36px;
        height: 36px;
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        background: rgba(255, 255, 255, 0.02);
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
    }

    :global([data-theme="light"]) .btn-icon {
        border-color: rgba(0, 0, 0, 0.06);
        background: rgba(0, 0, 0, 0.02);
        color: var(--text-secondary);
    }

    .btn-icon.warning:hover {
        background: rgba(245, 158, 11, 0.15);
        color: #f59e0b;
    }

    .btn-icon.danger:hover {
        background: rgba(239, 68, 68, 0.12);
        color: #ef4444;
    }

    .btn-icon.success:hover {
        background: rgba(16, 185, 129, 0.12);
        color: #10b981;
    }

    .actions {
        display: flex;
        gap: 0.5rem;
    }

    .cards-wrapper {
        padding: 0 1.25rem 1rem 1.25rem;
    }

    .user-cards {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 0.9rem;
        margin-top: 0.25rem;
    }

    .user-card {
        background: linear-gradient(145deg, var(--bg-surface), #0b0c10);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 16px;
        overflow: hidden;
        box-shadow: 0 10px 28px rgba(0, 0, 0, 0.22);
    }

    :global([data-theme="light"]) .user-card {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 12px 32px rgba(0, 0, 0, 0.08),
            0 0 0 1px rgba(255, 255, 255, 0.8);
    }

    .card-top {
        display: flex;
        justify-content: space-between;
        gap: 0.75rem;
        align-items: flex-start;
        padding: 1rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.06);
        background: rgba(255, 255, 255, 0.015);
    }

    :global([data-theme="light"]) .card-top {
        border-bottom-color: rgba(0, 0, 0, 0.06);
        background: rgba(0, 0, 0, 0.01);
    }

    .user-meta {
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 0.1rem;
    }

    .user-email {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        max-width: 220px;
    }

    .card-bottom {
        padding: 1rem;
    }

    .meta-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 0.85rem 0.9rem;
    }

    .meta-item {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        min-width: 0;
    }

    .meta-label {
        font-size: 0.78rem;
        color: var(--text-secondary);
    }

    .meta-value {
        font-weight: 650;
        color: var(--text-primary);
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .cards-pagination {
        margin-top: 0.75rem;
    }

    .empty-state-container {
        padding: 2.25rem 1rem;
        text-align: center;
        color: var(--text-secondary);
    }

    .empty-icon {
        opacity: 0.6;
        margin-bottom: 0.75rem;
    }

    .empty-state-container h3 {
        color: var(--text-primary);
        margin: 0.25rem 0 0.35rem 0;
    }

    .error-state {
        padding: 2rem 1.25rem;
        text-align: center;
        color: var(--text-secondary);
    }

    .error-state p {
        margin: 0.75rem 0 0 0;
        color: var(--text-secondary);
    }

    .details-grid {
        display: grid;
        grid-template-columns: 1fr;
        gap: 0.9rem;
    }

    @media (min-width: 720px) {
        .details-grid {
            grid-template-columns: 1fr 1fr;
        }

        .details-grid :global(.detail-card:nth-child(3)) {
            grid-column: 1 / -1;
        }
    }

    .detail-card {
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 14px;
        padding: 1rem;
    }

    :global([data-theme="light"]) .detail-card {
        background: rgba(0, 0, 0, 0.02);
        border-color: rgba(0, 0, 0, 0.06);
    }

    .detail-title {
        font-weight: 800;
        color: var(--text-primary);
        margin-bottom: 0.75rem;
    }

    .detail-row {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 1rem;
        padding: 0.5rem 0;
        border-top: 1px solid rgba(255, 255, 255, 0.06);
    }

    :global([data-theme="light"]) .detail-row {
        border-top-color: rgba(0, 0, 0, 0.06);
    }

    .detail-row:first-of-type {
        border-top: none;
        padding-top: 0;
    }

    .detail-key {
        color: var(--text-secondary);
        font-size: 0.9rem;
        white-space: nowrap;
    }

    .detail-val {
        color: var(--text-primary);
        font-weight: 650;
        text-align: right;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    @media (max-width: 900px) {
        .stats-row {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }
    }

    @media (max-width: 768px) {
        .stats-row {
            grid-template-columns: 1fr;
            gap: 0.75rem;
        }

        .toolbar-wrapper {
            padding: 0.9rem 1rem 0 1rem;
        }

        .table-wrapper {
            padding: 0 1rem 1rem 1rem;
        }

        .cards-wrapper {
            padding: 0 1rem 1rem 1rem;
        }

        .user-cards {
            grid-template-columns: 1fr;
        }

        .meta-grid {
            grid-template-columns: 1fr;
        }

        .user-email {
            max-width: 180px;
        }
    }
</style>
