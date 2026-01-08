<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api/client';
    import { token, user } from '$lib/stores/auth';
    import { theme } from '$lib/stores/theme';
    import { appSettings } from '$lib/stores/settings';
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

    // Visibility States
    let showCurrentPassword = false;
    let showNewPassword = false;
    let showConfirmPassword = false;

    // Default policy if store not loaded yet
    $: policy = $appSettings.auth || {
        password_min_length: 8,
        password_require_uppercase: true,
        password_require_number: true,
        password_require_special: false
    };

    // Load initial data
    onMount(async () => {
        if (!$token) {
            goto('/login');
            return;
        }

        await appSettings.init();

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

    function validatePassword(pwd: string): string | null {
        if (pwd.length < policy.password_min_length) {
            return `Password must be at least ${policy.password_min_length} characters`;
        }
        if (policy.password_require_uppercase && !/[A-Z]/.test(pwd)) {
            return "Password must contain at least one uppercase letter";
        }
        if (policy.password_require_number && !/[0-9]/.test(pwd)) {
            return "Password must contain at least one number";
        }
        if (policy.password_require_special && !/[!@#$%^&*()_+\-=[\]{}|;:',.<>?/`~]/.test(pwd)) {
            return "Password must contain at least one special character";
        }
        return null;
    }

    // Change Password
    async function changePassword() {
        if (!$token) return;
        
        if (passwordData.new !== passwordData.confirm) {
            showMessage('error', 'New passwords do not match');
            return;
        }

        const policyError = validatePassword(passwordData.new);
        if (policyError) {
             showMessage('error', policyError);
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
        ? profileData.name
            .split(' ')
            .map(n => n[0])
            .slice(0, 2)
            .join('')
            .toUpperCase()
        : 'U';

    const tabs = [
        { id: 'general', label: 'General', icon: 'profile' },
        { id: 'security', label: 'Security', icon: 'lock' },
        { id: 'preferences', label: 'Preferences', icon: 'settings' }
    ];
</script>

<div class="page-container fade-in">
    <div class="header-section">
        <h1 class="page-title">Settings</h1>
        <p class="page-subtitle">Manage your account settings and preferences.</p>
    </div>

    {#if message.text}
        <div class="alert alert-{message.type} slide-in">
            <Icon name={message.type === 'success' ? 'check' : 'alert'} size={20} />
            <span>{message.text}</span>
        </div>
    {/if}

    <div class="layout-grid">
        <!-- Sidebar Navigation -->
        <aside class="sidebar">
            <div class="user-mini-profile">
                <div class="avatar-circle">{initials}</div>
                <div class="user-info">
                    <span class="name">{profileData.name || 'User'}</span>
                    <span class="role">{profileData.role || 'Member'}</span>
                </div>
            </div>
            
            <nav class="nav-menu">
                {#each tabs as tab}
                    <button 
                        class="nav-item {activeTab === tab.id ? 'active' : ''}" 
                        on:click={() => activeTab = tab.id}
                    >
                        <Icon name={tab.icon} size={18} />
                        <span>{tab.label}</span>
                        {#if activeTab === tab.id}
                            <div class="active-indicator" />
                        {/if}
                    </button>
                {/each}
            </nav>
        </aside>

        <!-- Main Content Area -->
        <main class="content-area">
            {#if activeTab === 'general'}
                <div class="card section fade-in-up">
                    <div class="card-header">
                        <div>
                            <h2 class="card-title">Profile Information</h2>
                            <p class="card-subtitle">Update your account details and public profile.</p>
                        </div>
                    </div>
                    
                    <div class="profile-header-edit">
                        <div class="avatar-large-wrapper">
                            <div class="avatar-large">{initials}</div>
                            <button class="avatar-edit-btn" title="Change Avatar">
                                <Icon name="camera" size={16} />
                            </button>
                        </div>
                        <div class="profile-header-text">
                            <h3>{profileData.name || 'Your Name'}</h3>
                            <p>{profileData.role || 'Role'}</p>
                        </div>
                    </div>

                    <form on:submit|preventDefault={saveProfile} class="settings-form">
                        <div class="form-group">
                            <label class="form-label" for="full-name">Display Name</label>
                            <input 
                                type="text" 
                                id="full-name" 
                                class="form-input" 
                                placeholder="Enter your full name"
                                bind:value={profileData.name} 
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="email">Email Address</label>
                            <input 
                                type="email" 
                                id="email" 
                                class="form-input" 
                                placeholder="name@example.com"
                                bind:value={profileData.email} 
                            />
                        </div>

                        <div class="form-actions">
                            <button type="submit" class="btn btn-primary" disabled={loading}>
                                {#if loading}
                                    <span class="spinner"></span> Saving...
                                {:else}
                                    Save Changes
                                {/if}
                            </button>
                        </div>
                    </form>
                </div>
            {/if}

            {#if activeTab === 'security'}
                <div class="card section fade-in-up">
                    <div class="card-header">
                        <h2 class="card-title">Password & Security</h2>
                        <p class="card-subtitle">Manage your password and security settings.</p>
                    </div>

                    <form on:submit|preventDefault={changePassword} class="settings-form">
                        <div class="form-group">
                            <label class="form-label" for="current-pass">Current Password</label>
                            <div class="input-wrapper">
                                <input 
                                    type={showCurrentPassword ? "text" : "password"} 
                                    id="current-pass" 
                                    class="form-input" 
                                    placeholder="••••••••"
                                    bind:value={passwordData.current} 
                                />
                                <button 
                                    type="button" 
                                    class="toggle-password" 
                                    on:click={() => showCurrentPassword = !showCurrentPassword}
                                    tabindex="-1"
                                >
                                    <Icon name={showCurrentPassword ? 'eye-off' : 'eye'} size={18} />
                                </button>
                            </div>
                        </div>
                        
                        <div class="grid-2">
                            <div class="form-group">
                                <label class="form-label" for="new-pass">New Password</label>
                                <div class="input-wrapper">
                                    <input 
                                        type={showNewPassword ? "text" : "password"} 
                                        id="new-pass" 
                                        class="form-input" 
                                        placeholder="••••••••"
                                        bind:value={passwordData.new} 
                                    />
                                    <button 
                                        type="button" 
                                        class="toggle-password" 
                                        on:click={() => showNewPassword = !showNewPassword}
                                        tabindex="-1"
                                    >
                                        <Icon name={showNewPassword ? 'eye-off' : 'eye'} size={18} />
                                    </button>
                                </div>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="confirm-pass">Confirm Password</label>
                                <div class="input-wrapper">
                                    <input 
                                        type={showConfirmPassword ? "text" : "password"} 
                                        id="confirm-pass" 
                                        class="form-input" 
                                        placeholder="••••••••"
                                        bind:value={passwordData.confirm} 
                                    />
                                    <button 
                                        type="button" 
                                        class="toggle-password" 
                                        on:click={() => showConfirmPassword = !showConfirmPassword}
                                        tabindex="-1"
                                    >
                                        <Icon name={showConfirmPassword ? 'eye-off' : 'eye'} size={18} />
                                    </button>
                                </div>
                            </div>
                        </div>

                        <div class="password-requirements">
                            <p>Password requirements:</p>
                            <ul>
                                <li class={passwordData.new.length >= policy.password_min_length ? 'valid' : ''}>
                                    At least {policy.password_min_length} characters long
                                </li>
                                {#if policy.password_require_uppercase}
                                    <li class={/[A-Z]/.test(passwordData.new) ? 'valid' : ''}>
                                        One uppercase letter
                                    </li>
                                {/if}
                                {#if policy.password_require_number}
                                    <li class={/[0-9]/.test(passwordData.new) ? 'valid' : ''}>
                                        One number
                                    </li>
                                {/if}
                                {#if policy.password_require_special}
                                    <li class={/[!@#$%^&*()_+\-=[\]{}|;:',.<>?/`~]/.test(passwordData.new) ? 'valid' : ''}>
                                        One special character
                                    </li>
                                {/if}
                            </ul>
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
                <div class="card section fade-in-up">
                    <div class="card-header">
                        <h2 class="card-title">Appearance</h2>
                        <p class="card-subtitle">Customize the look and feel of the application.</p>
                    </div>

                    <div class="setting-item">
                        <div class="setting-info">
                            <h3>Dark Mode</h3>
                            <p>Toggle between dark and light themes.</p>
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
    /* Layout & Containers */
    .page-container {
        padding: 2rem;
        max-width: 1100px;
        margin: 0 auto;
        min-height: 100%;
    }

    .header-section {
        margin-bottom: 2.5rem;
    }

    .page-title {
        font-size: 1.875rem;
        font-weight: 700;
        color: var(--text-primary);
        letter-spacing: -0.025em;
    }

    .page-subtitle {
        color: var(--text-secondary);
        font-size: 1rem;
        margin-top: 0.5rem;
    }

    .layout-grid {
        display: grid;
        grid-template-columns: 260px 1fr;
        gap: 2.5rem;
        align-items: start;
    }

    /* Sidebar */
    .sidebar {
        position: sticky;
        top: 2rem;
    }

    .user-mini-profile {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 1rem;
        margin-bottom: 1.5rem;
        background: var(--bg-surface);
        border: 1px solid var(--border-subtle);
        border-radius: var(--radius-md);
    }

    .avatar-circle {
        width: 40px;
        height: 40px;
        border-radius: 50%;
        background: linear-gradient(135deg, var(--color-primary), var(--color-primary-hover));
        color: white;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 600;
        font-size: 1rem;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    }

    .user-info {
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .user-info .name {
        font-weight: 600;
        font-size: 0.95rem;
        color: var(--text-primary);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .user-info .role {
        font-size: 0.75rem;
        color: var(--text-secondary);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        font-weight: 500;
    }

    .nav-menu {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    .nav-item {
        position: relative;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        width: 100%;
        padding: 0.75rem 1rem;
        background: transparent;
        border: none;
        color: var(--text-secondary);
        font-size: 0.9rem;
        font-weight: 500;
        cursor: pointer;
        border-radius: var(--radius-md);
        transition: all 0.2s ease;
        text-align: left;
    }

    .nav-item:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .nav-item.active {
        background: var(--bg-surface);
        color: var(--color-primary);
        font-weight: 600;
        box-shadow: var(--shadow-sm);
    }

    .active-indicator {
        position: absolute;
        left: 0;
        top: 50%;
        transform: translateY(-50%);
        width: 3px;
        height: 16px;
        background: var(--color-primary);
        border-radius: 0 2px 2px 0;
    }

    /* Content Cards */
    .card {
        background: var(--bg-surface);
        border: 1px solid var(--border-subtle);
        border-radius: var(--radius-lg);
        padding: 2rem;
        box-shadow: var(--shadow-sm);
    }

    .card-header {
        margin-bottom: 2rem;
        padding-bottom: 1rem;
        border-bottom: 1px solid var(--border-subtle);
    }

    .card-title {
        font-size: 1.25rem;
        font-weight: 600;
        color: var(--text-primary);
        margin-bottom: 0.25rem;
    }

    .card-subtitle {
        font-size: 0.875rem;
        color: var(--text-secondary);
    }

    /* Profile Header Edit */
    .profile-header-edit {
        display: flex;
        align-items: center;
        gap: 1.5rem;
        margin-bottom: 2.5rem;
    }

    .avatar-large-wrapper {
        position: relative;
    }

    .avatar-large {
        width: 80px;
        height: 80px;
        border-radius: 50%;
        background: linear-gradient(135deg, var(--color-primary), var(--color-primary-hover));
        color: white;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 2rem;
        font-weight: 700;
        border: 4px solid var(--bg-surface);
        box-shadow: 0 0 0 1px var(--border-subtle);
    }

    .avatar-edit-btn {
        position: absolute;
        bottom: 0;
        right: 0;
        width: 28px;
        height: 28px;
        border-radius: 50%;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        display: flex;
        align-items: center;
        justify-content: center;
        color: var(--text-secondary);
        cursor: pointer;
        transition: all 0.2s;
        box-shadow: var(--shadow-sm);
    }

    .avatar-edit-btn:hover {
        color: var(--color-primary);
        border-color: var(--color-primary);
    }

    /* Forms */
    .settings-form {
        max-width: 100%;
    }

    .form-group {
        margin-bottom: 1.5rem;
    }

    .form-label {
        display: block;
        margin-bottom: 0.5rem;
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-primary);
    }

    .form-input {
        width: 100%;
        padding: 0.75rem 1rem;
        background: var(--bg-app); /* Slightly darker/lighter than surface to look indented */
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
        color: var(--text-primary);
        font-size: 0.95rem;
        transition: all 0.2s;
    }

    .form-input:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 2px var(--color-primary-subtle);
        background: var(--bg-surface);
    }

    .grid-2 {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1.5rem;
    }

    .form-actions {
        margin-top: 2.5rem;
        display: flex;
        justify-content: flex-end;
        border-top: 1px solid var(--border-subtle);
        padding-top: 1.5rem;
    }

    .btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
        padding: 0.6rem 1.25rem;
        border-radius: var(--radius-md);
        font-weight: 500;
        font-size: 0.9rem;
        cursor: pointer;
        transition: all 0.2s;
        border: none;
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }

    .btn-primary:hover:not(:disabled) {
        background: var(--color-primary-hover);
        transform: translateY(-1px);
    }
    
    .btn-primary:disabled {
        opacity: 0.7;
        cursor: not-allowed;
    }

    /* Settings Items */
    .setting-item {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1.5rem 0;
        border-bottom: 1px solid var(--border-subtle);
    }
    
    .setting-item:last-child {
        border-bottom: none;
    }

    .setting-info h3 {
        font-size: 1rem;
        font-weight: 500;
        margin-bottom: 0.25rem;
        color: var(--text-primary);
    }

    .setting-info p {
        color: var(--text-secondary);
        font-size: 0.875rem;
    }

    /* Toggle Switch */
    .toggle {
        position: relative;
        display: inline-block;
        width: 44px;
        height: 24px;
    }

    .toggle input { opacity: 0; width: 0; height: 0; }

    .slider {
        position: absolute;
        cursor: pointer;
        top: 0; left: 0; right: 0; bottom: 0;
        background-color: var(--bg-active);
        transition: .3s;
        border-radius: 24px;
    }

    .slider:before {
        position: absolute;
        content: "";
        height: 18px;
        width: 18px;
        left: 3px;
        bottom: 3px;
        background-color: white;
        transition: .3s;
        border-radius: 50%;
        box-shadow: 0 1px 2px rgba(0,0,0,0.2);
    }

    input:checked + .slider {
        background-color: var(--color-primary);
    }

    input:checked + .slider:before {
        transform: translateX(20px);
    }

    /* Alerts */
    .alert {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 1rem;
        margin-bottom: 2rem;
        border-radius: var(--radius-md);
        font-size: 0.9rem;
        font-weight: 500;
    }

    .alert-success {
        background: rgba(16, 185, 129, 0.1);
        border: 1px solid rgba(16, 185, 129, 0.2);
        color: var(--color-success);
    }

    .alert-error {
        background: rgba(239, 68, 68, 0.1);
        border: 1px solid rgba(239, 68, 68, 0.2);
        color: var(--color-danger);
    }

    .password-requirements {
        margin-top: 1rem;
        padding: 1rem;
        background: var(--bg-app);
        border-radius: var(--radius-md);
        font-size: 0.85rem;
        color: var(--text-secondary);
    }
    
    .password-requirements ul {
        margin-top: 0.5rem;
        padding-left: 1.25rem;
    }

    .valid {
        color: var(--color-success);
    }

    /* Input Password Toggle */
    .input-wrapper {
        position: relative;
    }

    .toggle-password {
        position: absolute;
        right: 12px;
        top: 50%;
        transform: translateY(-50%);
        background: none;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0;
        display: flex;
        align-items: center;
        transition: color 0.2s;
    }

    .toggle-password:hover {
        color: var(--text-primary);
    }

    .input-wrapper .form-input {
        padding-right: 40px;
    }

    .fade-in { animation: fadeIn 0.4s ease-out; }
    .fade-in-up { animation: fadeInUp 0.4s ease-out; }

    /* Spinner */
    .spinner {
        width: 1rem;
        height: 1rem;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-radius: 50%;
        border-top-color: white;
        animation: spin 0.8s linear infinite;
        display: inline-block;
    }

    @keyframes spin { to { transform: rotate(360deg); } }

    /* Responsive */
    @media (max-width: 768px) {
        .layout-grid {
            grid-template-columns: 1fr;
            gap: 1.5rem;
        }
        
        .sidebar {
            position: static;
        }

        .nav-menu {
            flex-direction: row;
            overflow-x: auto;
            padding-bottom: 0.5rem;
        }
        
        .nav-item {
            width: auto;
            white-space: nowrap;
        }

        .grid-2 {
            grid-template-columns: 1fr;
        }
    }
</style>