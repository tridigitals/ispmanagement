<script lang="ts">
    import {
        LayoutDashboard,
        Users,
        Settings,
        Shield,
        User,
        LogOut,
        ChevronUp,
        ChevronDown,
        ArrowLeft,
        Search,
        Bell,
        HelpCircle,
        Database,
        Rocket,
        Mail,
        Lock as LockIcon,
        Globe,
        Menu,
        PanelLeft,
        Activity,
        Calendar,
        CheckCircle,
        AlertCircle,
        ArrowRight,
        Camera,
        Eye,
        EyeOff
    } from 'lucide-svelte';

    export let name: string;
    export let size = 18;
    export let strokeWidth = 2;
    export let color = 'currentColor';

    // Mapping string name to Lucide Component
    const icons: Record<string, any> = {
        'dashboard': LayoutDashboard,
        'users': Users,
        'settings': Settings,
        'shield': Shield,
        'profile': User,
        'logout': LogOut,
        'chevron-up': ChevronUp,
        'chevron-down': ChevronDown,
        'arrow-left': ArrowLeft,
        'search': Search,
        'bell': Bell,
        'help-circle': HelpCircle,
        'database': Database,
        'app': Rocket,
        'mail': Mail,
        'lock': LockIcon,
        'global': Globe,
        'menu': Menu,
        'sidebar-toggle': PanelLeft,
        'activity': Activity,
        'calendar': Calendar,
        'check': CheckCircle,
        'alert': AlertCircle,
        'arrow-right': ArrowRight,
        'camera': Camera,
        'eye': Eye,
        'eye-off': EyeOff
    };

    $: IconComponent = icons[name] || HelpCircle; // Fallback to HelpCircle if not found
</script>

<svelte:component 
    this={IconComponent} 
    {size} 
    {strokeWidth} 
    {color} 
    class={$$props.class}
/>