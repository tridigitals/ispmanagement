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
        ChevronLeft,
        ChevronRight,
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
        EyeOff,
        Plus,
        Edit,
        Trash2,
        X,
        Slash,
        Info,
        Image,
        FileText,
        Film,
        Music,
        Archive,
        File,
        RefreshCw,
        Grid,
        List,
        HardDrive,
        Folder,
        Download,
        CreditCard,
        Server,
    } from "lucide-svelte";

    export let name: string;
    export let size = 18;
    export let strokeWidth = 2;
    export let color = "currentColor";

    // Mapping string name to Lucide Component
    const icons: Record<string, any> = {
        dashboard: LayoutDashboard,
        users: Users,
        settings: Settings,
        shield: Shield,
        profile: User,
        logout: LogOut,
        "chevron-up": ChevronUp,
        "chevron-down": ChevronDown,
        "chevron-left": ChevronLeft,
        "chevron-right": ChevronRight,
        "arrow-left": ArrowLeft,
        search: Search,
        bell: Bell,
        "help-circle": HelpCircle,
        database: Database,
        app: Rocket,
        mail: Mail,
        lock: LockIcon,
        global: Globe,
        menu: Menu,
        "sidebar-toggle": PanelLeft,
        activity: Activity,
        calendar: Calendar,
        check: CheckCircle, // Alias
        "check-circle": CheckCircle,
        alert: AlertCircle, // Alias
        "alert-circle": AlertCircle,
        "arrow-right": ArrowRight,
        camera: Camera,
        eye: Eye,
        "eye-off": EyeOff,
        plus: Plus,
        edit: Edit,
        trash: Trash2,
        "trash-2": Trash2,
        x: X,
        slash: Slash,
        info: Info,
        image: Image,
        "file-text": FileText,
        film: Film,
        music: Music,
        archive: Archive,
        file: File,
        "refresh-cw": RefreshCw,
        grid: Grid,
        list: List,
        "hard-drive": HardDrive,
        folder: Folder,
        download: Download,
        "credit-card": CreditCard,
        server: Server,
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
