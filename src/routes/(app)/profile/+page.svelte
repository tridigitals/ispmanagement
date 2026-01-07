<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api/client';
    import { token, user } from '$lib/stores/auth';
    import { theme } from '$lib/stores/theme';
    import { goto } from '$app/navigation';
    import Icon from '$lib/components/Icon.svelte';

    let activeTab = 'general';
    let loading = false;
    let message = { type: '', text: '' };

    // User Data State (for editing)
    let profileData = {
        id: '',
        name: '',
        email: '',
        role: ''
    };

    // Password State
    let passwordData = {
        current: '',
        new: '',
        confirm: ''
    };

    // Load initial data
    onMount(async () => {
        if (!$token) {
            goto('/login');
            return;
        }

        // Initialize profile data from store
        if ($user) {
            profileData = {
                id: $user.id,
                name: $user.name,
                email: $user.email,
                role: $user.role
            };
        }
    });

    // Helper to show messages
    function showMessage(type: 'success' | 'error', text: string) {
        message = { type, text };
        setTimeout(() => message = { type: '', text: '' }, 4000);
    }

    // Save Profile
    async function saveProfile() {
        if (!$token) return;
        loading = true;
        try {
            await api.users.update(profileData.id, {
                name: profileData.name,
                email: profileData.email
            });
            
            user.update(u => u ? { ...u, name: profileData.name, email: profileData.email } : null);
            showMessage('success', 'Profile updated successfully!');
        } catch (error: any) {
            console.error(error);
            showMessage('error', error.toString() || 'Failed to update profile');
        } finally {
            loading = false;
        }
    }

    // Change Password
    async function changePassword() {
        if (!$token) return;
        
        if (passwordData.new !== passwordData.confirm) {
            showMessage('error', 'New passwords do not match');
            return;
        }

        if (passwordData.new.length < 8) {
             showMessage('error', 'Password must be at least 8 characters');
             return;
        }

        loading = true;
        try {
            await api.auth.changePassword(
                $token,
                passwordData.current,
                passwordData.new
            );
            
            showMessage('success', 'Password changed successfully!');
            passwordData = { current: '', new: '', confirm: '' }; 
        } catch (error: any) {
            console.error(error);
            showMessage('error', error.toString() || 'Failed to change password');
        } finally {
            loading = false;
        }
    }

    // Get initials for avatar
    $: initials = profileData.name
        .split(' ')
        .map(n => n[0])
        .slice(0, 2)
        .join('')
        .toUpperCase();

    const tabs = [
        { id: 'general', label: 'Profile', icon: 'profile' },
        { id: 'security', label: 'Security', icon: 'lock' },
        { id: 'preferences', label: 'Preferences', icon: 'settings' }
    ];
</script>

<div class="settings-container fade-in">
    <div class="header-section">
        <h1>My Profile</h1>
        <p class="subtitle">Manage your personal information and preferences.</p>
    </div>

    {#if message.text}
        <div class="alert alert-{message.type} slide-in">
            {message.text}
        </div>
    {/if}

    <div class="layout-grid">
        <aside class="sidebar card">
            <div class="user-mini-profile">
                <div class="avatar-circle">{initials}</div>
                <div class="user-info">
                    <span class="name">{profileData.name || 'User'}</span>
                    <span class="role">{profileData.role || 'Member'}</span>
                </div>
            </div>
            
            <nav>
                {#each tabs as tab}
                    <button 
                        class="nav-item {activeTab === tab.id ? 'active' : ''}" 
                        on:click={() => activeTab = tab.id}
                    >
                        <Icon name={tab.icon} size={18} />
                        {tab.label}
                    </button>
                {/each}
            </nav>
        </aside>

        <main class="content">
            {#if activeTab === 'general'}
                <div class="card section fade-in">
                    <div class="card-header">
                        <h2 class="card-title">Profile Information</h2>
                        <p class="card-subtitle">Update your account details and public profile.</p>
                    </div>
                    
                    <div class="profile-header-edit">
                        <div class="avatar-large">{initials}</div>
                        <button class="btn btn-secondary btn-sm">Change Avatar</button>
                    </div>

                    <form on:submit|preventDefault={saveProfile} class="settings-form">
                        <div class="form-group">
                            <label class="form-label" for="full-name">Full Name</label>
                            <input type="text" id="full-name" class="form-input" bind:value={profileData.name} />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="email">Email Address</label>
                            <input type="email" id="email" class="form-input" bind:value={profileData.email} />
                        </div>

                        <div class="form-actions">
                            <button type="submit" class="btn btn-primary" disabled={loading}>
                                {loading ? 'Saving...' : 'Save Changes'}
                            </button>
                        </div>
                    </form>
                </div>
            {/if}

            {#if activeTab === 'security'}
                <div class="card section fade-in">
                    <div class="card-header">
                        <h2 class="card-title">Password & Security</h2>
                        <p class="card-subtitle">Manage your password and security settings.</p>
                    </div>

                    <form on:submit|preventDefault={changePassword} class="settings-form">
                        <div class="form-group">
                            <label class="form-label" for="current-pass">Current Password</label>
                            <input type="password" id="current-pass" class="form-input" bind:value={passwordData.current} />
                        </div>
                        
                        <div class="grid-2">
                            <div class="form-group">
                                <label class="form-label" for="new-pass">New Password</label>
                                <input type="password" id="new-pass" class="form-input" bind:value={passwordData.new} />
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="confirm-pass">Confirm Password</label>
                                <input type="password" id="confirm-pass" class="form-input" bind:value={passwordData.confirm} />
                            </div>
                        </div>

                        <div class="form-actions">
                            <button type="submit" class="btn btn-primary" disabled={loading}>
                                {loading ? 'Updating...' : 'Update Password'}
                            </button>
                        </div>
                    </form>
                </div>
            {/if}

            {#if activeTab === 'preferences'}
                <div class="card section fade-in">
                    <div class="card-header">
                        <h2 class="card-title">Appearance</h2>
                        <p class="card-subtitle">Customize the look and feel of the application.</p>
                    </div>

                    <div class="setting-item">
                        <div class="setting-info">
                            <h3>Dark Mode</h3>
                            <p>Enable or disable dark mode for the entire application.</p>
                        </div>
                        <label class="toggle">
                            <input 
                                type="checkbox" 
                                checked={$theme === 'dark'} 
                                on:change={() => theme.toggle()}
                            >
                            <span class="slider"></span>
                        </label>
                    </div>
                </div>
            {/if}
        </main>
    </div>
</div>

<style>
    .settings-container { padding: 2rem; max-width: 1200px; margin: 0 auto; }
    .header-section { margin-bottom: 2rem; }
    .subtitle { color: var(--text-secondary); font-size: 1.1rem; margin-top: 0.5rem; }
    .layout-grid { display: grid; grid-template-columns: 280px 1fr; gap: 2rem; align-items: start; }
    
    .sidebar { padding: 1.5rem; position: sticky; top: 2rem; }
    .user-mini-profile { display: flex; align-items: center; gap: 1rem; padding-bottom: 1.5rem; margin-bottom: 1.5rem; border-bottom: 1px solid var(--border-color); }
    .avatar-circle { width: 48px; height: 48px; border-radius: 50%; background: var(--color-primary); color: white; display: flex; align-items: center; justify-content: center; font-weight: bold; }
    .user-info { display: flex; flex-direction: column; overflow: hidden; }
    .user-info .name { font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .user-info .role { font-size: 0.8rem; color: var(--text-secondary); text-transform: capitalize; }

    .nav-item { display: flex; align-items: center; gap: 0.75rem; width: 100%; padding: 0.75rem 1rem; background: transparent; border: none; color: var(--text-secondary); font-size: 0.95rem; font-weight: 500; cursor: pointer; border-radius: var(--border-radius-sm); transition: all 0.2s; text-align: left; }
    .nav-item:hover { background: rgba(255, 255, 255, 0.05); color: var(--text-primary); }
    .nav-item.active { background: var(--color-primary); color: white; }

    .profile-header-edit { display: flex; align-items: center; gap: 1.5rem; margin-bottom: 2rem; padding-bottom: 2rem; border-bottom: 1px solid var(--border-color); }
    .avatar-large { width: 80px; height: 80px; border-radius: 50%; background: var(--color-primary); color: white; display: flex; align-items: center; justify-content: center; font-size: 2rem; font-weight: bold; }
    .settings-form { max-width: 600px; }
    .grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
    .form-actions { margin-top: 2rem; display: flex; justify-content: flex-end; }
    .btn-sm { padding: 0.5rem 1rem; font-size: 0.8rem; }

    .setting-item { display: flex; justify-content: space-between; align-items: center; padding: 1.25rem 0; border-bottom: 1px solid var(--border-color); }
    .setting-info h3 { font-size: 1rem; margin-bottom: 0.25rem; }
    .setting-info p { color: var(--text-secondary); font-size: 0.875rem; }

    .toggle { position: relative; display: inline-block; width: 50px; height: 26px; }
    .toggle input { opacity: 0; width: 0; height: 0; }
    .slider { position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: var(--bg-tertiary); transition: .4s; border-radius: 34px; border: 1px solid var(--border-color); }
    .slider:before { position: absolute; content: ""; height: 18px; width: 18px; left: 3px; bottom: 3px; background-color: var(--text-secondary); transition: .4s; border-radius: 50%; }
    input:checked + .slider { background-color: var(--color-primary); border-color: var(--color-primary); }
    input:checked + .slider:before { transform: translateX(24px); background-color: white; }

    .alert { padding: 1rem; margin-bottom: 1.5rem; border-radius: var(--border-radius-sm); font-weight: 500; }
    .alert-success { background: rgba(34, 197, 94, 0.1); border: 1px solid rgba(34, 197, 94, 0.2); color: #4ade80; }
    .alert-error { background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; }

    @keyframes fadeIn { from { opacity: 0; transform: translateY(10px); } to { opacity: 1; transform: translateY(0); } }
    .fade-in { animation: fadeIn 0.4s ease-out; }

    @media (max-width: 768px) {
        .layout-grid { grid-template-columns: 1fr; }
        .sidebar { position: static; margin-bottom: 1rem; }
    }
</style>