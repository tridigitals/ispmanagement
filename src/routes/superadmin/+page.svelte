<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import { user } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import { fade, fly } from "svelte/transition";
    import Icon from "$lib/components/Icon.svelte";

    let tenants: any[] = [];
    let loading = true;
    let error = "";
    
    // Modal state
    let showCreateModal = false;
    let newTenant = { name: "", slug: "", ownerEmail: "", ownerPassword: "" };
    let creating = false;
    let showPassword = false;

    onMount(async () => {
        loadTenants();
    });

    async function loadTenants() {
        loading = true;
        try {
            const res = await api.superadmin.listTenants();
            tenants = res.data;
        } catch (e: any) {
            error = e.toString();
            if (e.toString().includes("Unauthorized")) {
                goto("/dashboard");
            }
        } finally {
            loading = false;
        }
    }

    async function createTenant() {
        if (!newTenant.name || !newTenant.slug || !newTenant.ownerEmail || !newTenant.ownerPassword) return;
        creating = true;
        try {
            await api.superadmin.createTenant(
                newTenant.name, 
                newTenant.slug, 
                newTenant.ownerEmail, 
                newTenant.ownerPassword
            );
            
            // Success! Close modal and refresh list
            showCreateModal = false;
            newTenant = { name: "", slug: "", ownerEmail: "", ownerPassword: "" };
            
            // Reload data immediately
            await loadTenants();
            
        } catch (e: any) {
            alert("Failed to create tenant: " + e);
        } finally {
            creating = false;
        }
    }

    function generateSlug() {
        if (!newTenant.name) return;
        newTenant.slug = newTenant.name
            .toLowerCase()
            .replace(/[^a-z0-9]+/g, '-')
            .replace(/(^-|-$)/g, '');
    }

    async function deleteTenant(id: string) {
        if (!confirm("Are you sure? This will delete the tenant and ALL its data permanently.")) return;
        
        try {
            await api.superadmin.deleteTenant(id);
            await loadTenants();
        } catch (e: any) {
            alert("Failed to delete: " + e);
        }
    }

    // Helper for initials
    const getInitials = (name: string) => name.substring(0, 2).toUpperCase();
</script>

<div class="stats-overview">
    <div class="stat-card gradient-1" in:fly={{ y: 20, delay: 100 }}>
            <div class="stat-icon"><Icon name="users" size={24} /></div>
            <div class="stat-info">
                <h3>Total Tenants</h3>
                <div class="value">{tenants.length}</div>
            </div>
        </div>
        <div class="stat-card gradient-2" in:fly={{ y: 20, delay: 200 }}>
            <div class="stat-icon"><Icon name="activity" size={24} /></div>
            <div class="stat-info">
                <h3>Active Revenue</h3>
                <div class="value">$0.00</div>
            </div>
        </div>
        <div class="stat-card gradient-3" in:fly={{ y: 20, delay: 300 }}>
            <div class="stat-icon"><Icon name="server" size={24} /></div>
            <div class="stat-info">
                <h3>System Health</h3>
                <div class="value status-ok">Operational</div>
            </div>
        </div>
    </div>

    <div class="content-card" in:fly={{ y: 20, delay: 400 }}>
        <div class="card-header">
            <div class="header-left">
                <h2>Tenant Directory</h2>
                <span class="count-badge">{tenants.length} Organizations</span>
            </div>
            <button class="btn btn-primary glow-effect" on:click={() => showCreateModal = true}>
                <Icon name="plus" size={18} />
                Deploy New Tenant
            </button>
        </div>

        {#if showCreateModal}
            <div class="modal-backdrop" on:click={() => showCreateModal = false} transition:fade={{ duration: 200 }}>
                <div class="modal-card" on:click|stopPropagation transition:fly={{ y: 20, duration: 300 }}>
                    <h3>Deploy New Organization</h3>
                    <form on:submit|preventDefault={createTenant}>
                        <div class="form-group">
                            <label>Organization Name</label>
                            <input 
                                type="text" 
                                bind:value={newTenant.name} 
                                on:input={generateSlug}
                                placeholder="e.g. Acme Corp" 
                                required 
                                autoFocus
                            />
                        </div>
                        <div class="form-group">
                            <label>URL Slug</label>
                            <div class="slug-input">
                                <span class="prefix">/</span>
                                <input type="text" bind:value={newTenant.slug} placeholder="acme-corp" required />
                            </div>
                        </div>

                        <div class="divider">Admin Account</div>

                        <div class="form-group">
                            <label>Owner Email</label>
                            <input type="email" bind:value={newTenant.ownerEmail} placeholder="admin@acme.com" required />
                        </div>

                        <div class="form-group">
                            <label>Owner Password</label>
                            <div class="password-input-wrapper">
                                <input 
                                    type={showPassword ? "text" : "password"} 
                                    bind:value={newTenant.ownerPassword} 
                                    placeholder="••••••••" 
                                    required 
                                />
                                <button type="button" class="eye-btn" on:click={() => showPassword = !showPassword}>
                                    <Icon name={showPassword ? 'eye-off' : 'eye'} size={16} />
                                </button>
                            </div>
                        </div>

                        <div class="modal-actions">
                            <button type="button" class="btn btn-glass" on:click={() => showCreateModal = false}>Cancel</button>
                            <button type="submit" class="btn btn-primary" disabled={creating}>
                                {creating ? 'Deploying...' : 'Deploy Tenant'}
                            </button>
                        </div>
                    </form>
                </div>
            </div>
        {/if}

        {#if loading}
            <div class="loading-state">
                <div class="spinner"></div>
                <p>Syncing data...</p>
            </div>
        {:else if error}
            <div class="error-state">
                <Icon name="alert-triangle" size={32} />
                <p>{error}</p>
            </div>
        {:else}
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>Organization</th>
                            <th>Domain / Slug</th>
                            <th>Status</th>
                            <th>Created At</th>
                            <th class="text-right">Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each tenants as tenant}
                            <tr class="fade-in-row">
                                <td>
                                    <div class="tenant-info">
                                        <div class="avatar">{getInitials(tenant.name)}</div>
                                        <div>
                                            <div class="tenant-name">{tenant.name}</div>
                                            <div class="tenant-id">ID: {tenant.id.substring(0, 8)}...</div>
                                        </div>
                                    </div>
                                </td>
                                <td>
                                    <div class="slug-pill">
                                        <span class="prefix">/</span>{tenant.slug}
                                    </div>
                                    {#if tenant.custom_domain}
                                        <div class="domain-link">
                                            <Icon name="link" size={12} /> {tenant.custom_domain}
                                        </div>
                                    {/if}
                                </td>
                                <td>
                                    {#if tenant.is_active}
                                        <span class="status-pill active">
                                            <span class="dot"></span> Active
                                        </span>
                                    {:else}
                                        <span class="status-pill inactive">
                                            <span class="dot"></span> Inactive
                                        </span>
                                    {/if}
                                </td>
                                <td class="text-muted">{new Date(tenant.created_at).toLocaleDateString()}</td>
                                <td class="text-right">
                                    <button class="action-btn danger" on:click={() => deleteTenant(tenant.id)} title="Delete Organization">
                                        <Icon name="trash" size={18} />
                                    </button>
                                </td>
                            </tr>
                        {:else}
                            <tr>
                                <td colspan="5" class="empty-state">
                                    <Icon name="search" size={48} />
                                    <h3>No Tenants Found</h3>
                                    <p>Get started by deploying your first organization.</p>
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    </div>

<style>
    /* Stats Grid */
    .stats-overview {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
        gap: 1.5rem;
        margin-bottom: 2.5rem;
    }

    .stat-card {
        background: #1e293b;
        padding: 1.75rem;
        border-radius: 16px;
        border: 1px solid rgba(255,255,255,0.05);
        display: flex;
        align-items: center;
        gap: 1.5rem;
        position: relative;
        overflow: hidden;
    }

    .stat-card::before {
        content: '';
        position: absolute;
        top: 0; left: 0; width: 100%; height: 100%;
        background: linear-gradient(135deg, rgba(255,255,255,0.03), transparent);
        pointer-events: none;
    }

    .stat-icon {
        width: 56px;
        height: 56px;
        border-radius: 14px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 1.5rem;
        background: rgba(255,255,255,0.05);
        color: #fff;
    }

    .gradient-1 .stat-icon { background: linear-gradient(135deg, #3b82f6, #2563eb); box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3); }
    .gradient-2 .stat-icon { background: linear-gradient(135deg, #10b981, #059669); box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3); }
    .gradient-3 .stat-icon { background: linear-gradient(135deg, #f59e0b, #d97706); box-shadow: 0 4px 12px rgba(245, 158, 11, 0.3); }

    .stat-info h3 {
        font-size: 0.9rem;
        color: #94a3b8;
        margin: 0 0 0.25rem 0;
        font-weight: 500;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .stat-info .value {
        font-size: 1.75rem;
        font-weight: 700;
        color: #fff;
    }

    /* Content Card */
    .content-card {
        background: #1e293b;
        border-radius: 16px;
        border: 1px solid rgba(255,255,255,0.05);
        box-shadow: 0 4px 24px rgba(0,0,0,0.2);
    }

    .card-header {
        padding: 1.5rem 2rem;
        border-bottom: 1px solid rgba(255,255,255,0.05);
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    .header-left {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .card-header h2 {
        font-size: 1.25rem;
        font-weight: 700;
        margin: 0;
        color: #fff;
    }

    .count-badge {
        background: rgba(255,255,255,0.1);
        color: #94a3b8;
        padding: 0.2rem 0.6rem;
        border-radius: 12px;
        font-size: 0.8rem;
        font-weight: 600;
    }

    /* Table */
    .table-responsive {
        width: 100%;
        overflow-x: auto;
    }

    .data-table {
        width: 100%;
        border-collapse: collapse;
    }

    .data-table th {
        text-align: left;
        padding: 1rem 2rem;
        font-size: 0.8rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: #64748b;
        font-weight: 600;
        border-bottom: 1px solid rgba(255,255,255,0.05);
        background: rgba(0,0,0,0.1);
    }

    .data-table td {
        padding: 1.25rem 2rem;
        border-bottom: 1px solid rgba(255,255,255,0.03);
        vertical-align: middle;
        color: #e2e8f0;
        font-size: 0.95rem;
    }

    .data-table tr:hover {
        background: rgba(255,255,255,0.02);
    }

    /* Tenant Info */
    .tenant-info {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .avatar {
        width: 40px;
        height: 40px;
        background: linear-gradient(135deg, #475569, #334155);
        border-radius: 10px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 700;
        color: white;
        font-size: 1rem;
        box-shadow: 0 2px 8px rgba(0,0,0,0.2);
    }

    .tenant-name {
        font-weight: 600;
        color: #fff;
    }

    .tenant-id {
        font-size: 0.75rem;
        color: #64748b;
        font-family: monospace;
    }

    /* Slug */
    .slug-pill {
        background: rgba(15, 23, 42, 0.5);
        border: 1px solid rgba(255,255,255,0.1);
        padding: 0.3rem 0.6rem;
        border-radius: 6px;
        display: inline-block;
        font-family: monospace;
        font-size: 0.85rem;
        color: #cbd5e1;
    }

    .prefix { color: #64748b; margin-right: 2px; }

    .domain-link {
        font-size: 0.8rem;
        color: #3b82f6;
        margin-top: 0.25rem;
        display: flex;
        align-items: center;
        gap: 0.25rem;
    }

    /* Status */
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
        color: #34d399;
        border: 1px solid rgba(16, 185, 129, 0.2);
    }

    .status-pill.inactive {
        background: rgba(239, 68, 68, 0.15);
        color: #f87171;
        border: 1px solid rgba(239, 68, 68, 0.2);
    }

    .dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: currentColor;
    }

    /* Buttons */
    .btn {
        display: flex; align-items: center; gap: 0.6rem;
        padding: 0.75rem 1.25rem;
        border-radius: 10px;
        font-weight: 600; cursor: pointer; border: none; font-size: 0.95rem;
        transition: all 0.2s;
    }

    .btn-glass {
        background: rgba(255,255,255,0.05);
        color: #cbd5e1;
        border: 1px solid rgba(255,255,255,0.1);
    }

    .btn-glass:hover {
        background: rgba(255,255,255,0.1);
        color: #fff;
    }

    .btn-primary {
        background: #6366f1;
        color: white;
        box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
    }

    .btn-primary:hover {
        background: #4f46e5;
        transform: translateY(-1px);
        box-shadow: 0 6px 16px rgba(99, 102, 241, 0.5);
    }

    .action-btn {
        width: 36px; height: 36px;
        border-radius: 8px;
        display: inline-flex; align-items: center; justify-content: center;
        border: none; background: transparent;
        color: #64748b; cursor: pointer;
        transition: all 0.2s;
    }

    .action-btn:hover { background: rgba(255,255,255,0.1); color: #fff; }
    .action-btn.danger:hover { background: rgba(239, 68, 68, 0.2); color: #ef4444; }

    .text-right { text-align: right; }
    .text-muted { color: #64748b; font-size: 0.9rem; }

    /* States */
    .empty-state {
        text-align: center;
        padding: 4rem;
        color: #64748b;
    }
    
    .empty-state h3 { color: #fff; margin: 1rem 0 0.5rem 0; }

    .loading-state, .error-state {
        padding: 4rem;
        text-align: center;
        color: #94a3b8;
    }

    .spinner {
        width: 24px; height: 24px;
        border: 3px solid rgba(255,255,255,0.1);
        border-top-color: #6366f1;
        border-radius: 50%;
        margin: 0 auto 1rem auto;
        animation: spin 1s linear infinite;
    }

    @keyframes spin { to { transform: rotate(360deg); } }

    /* Modal */
    .modal-backdrop {
        position: fixed;
        top: 0; left: 0; width: 100%; height: 100%;
        background: rgba(0,0,0,0.7);
        backdrop-filter: blur(4px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100;
    }

    .modal-card {
        background: #1e293b;
        padding: 2rem;
        border-radius: 16px;
        border: 1px solid rgba(255,255,255,0.1);
        width: 100%;
        max-width: 480px;
        box-shadow: 0 20px 50px rgba(0,0,0,0.5);
    }

    .modal-card h3 {
        margin: 0 0 1.5rem 0;
        font-size: 1.25rem;
        color: white;
    }

    .form-group {
        margin-bottom: 1.25rem;
    }

    .form-group label {
        display: block;
        margin-bottom: 0.5rem;
        font-size: 0.9rem;
        color: #94a3b8;
    }

    .form-group input {
        width: 100%;
        background: #0f172a;
        border: 1px solid rgba(255,255,255,0.1);
        padding: 0.75rem 1rem;
        border-radius: 8px;
        color: white;
        font-size: 1rem;
    }

    .form-group input:focus {
        outline: none;
        border-color: #6366f1;
        box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
    }

    .slug-input {
        position: relative;
    }

    .slug-input input {
        padding-left: 2rem;
    }

    .slug-input .prefix {
        position: absolute;
        left: 1rem;
        top: 50%;
        transform: translateY(-50%);
        color: #64748b;
    }

    .divider {
        margin: 1.5rem 0 1rem 0;
        display: flex;
        align-items: center;
        text-transform: uppercase;
        font-size: 0.75rem;
        font-weight: 700;
        color: #64748b;
        letter-spacing: 0.05em;
    }

    .divider::after {
        content: '';
        flex: 1;
        height: 1px;
        background: rgba(255,255,255,0.05);
        margin-left: 1rem;
    }

    .password-input-wrapper {
        position: relative;
    }

    .eye-btn {
        position: absolute;
        right: 0.75rem;
        top: 50%;
        transform: translateY(-50%);
        background: none;
        border: none;
        color: #64748b;
        cursor: pointer;
        display: flex;
        align-items: center;
    }

    .eye-btn:hover { color: white; }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 1rem;
        margin-top: 2rem;
    }
</style>
