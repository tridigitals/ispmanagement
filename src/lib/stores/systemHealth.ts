import { writable } from "svelte/store";

export interface TableInfo {
    name: string;
    row_count: number;
}

export interface RecentActivity {
    id: string;
    action: string;
    resource: string;
    user_email: string | null;
    created_at: string;
}

export interface SystemHealth {
    database: {
        is_connected: boolean;
        database_type: string;
        database_size_bytes: number;
        total_tables: number;
        tenants_count: number;
        users_count: number;
        audit_logs_count: number;
    };
    resources: {
        cpu_usage: number;
        memory_used_bytes: number;
        memory_total_bytes: number;
        os_name: string;
        os_version: string;
    };
    tables: TableInfo[];
    active_sessions: number;
    recent_activity: RecentActivity[];
    uptime_seconds: number;
    app_version: string;
    collected_at: string;
}

export const systemHealthCache = writable<{
    health: SystemHealth | null;
    fetchedAt: number;
}>({
    health: null,
    fetchedAt: 0,
});

