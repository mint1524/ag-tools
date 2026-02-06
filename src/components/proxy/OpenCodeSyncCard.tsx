import { useState, useEffect, useCallback } from 'react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
    CheckCircle2,
    AlertCircle,
    RefreshCw,
    Code,
    Loader2,
    Eye,
    RotateCcw,
    Copy,
    X
} from 'lucide-react';
import { copyToClipboard } from '../../utils/clipboard';
import { request as invoke } from '../../utils/request';
import { showToast } from '../common/ToastContainer';
import ModalDialog from '../common/ModalDialog';
import { cn } from '../../utils/cn';

interface OpenCodeSyncCardProps {
    proxyUrl: string;
    apiKey: string;
    className?: string;
}

interface OpenCodeStatus {
    installed: boolean;
    version: string | null;
    is_synced: boolean;
    has_backup: boolean;
    current_base_url: string | null;
    files: string[];
}

export const OpenCodeSyncCard = ({ proxyUrl, apiKey, className }: OpenCodeSyncCardProps) => {
    const { t } = useTranslation();
    const [status, setStatus] = useState<OpenCodeStatus | null>(null);
    const [loading, setLoading] = useState(false);
    const [syncing, setSyncing] = useState(false);
    const [syncAccounts, setSyncAccounts] = useState(false);
    const [viewingConfig, setViewingConfig] = useState<{
        content: string;
        fileName: string;
        allFiles: string[];
    } | null>(null);
    const [restoreConfirmOpen, setRestoreConfirmOpen] = useState(false);
    const [syncConfirmOpen, setSyncConfirmOpen] = useState(false);

    // Format proxy URL for OpenCode: trim trailing slashes, remove /v1 if present
    const getFormattedProxyUrl = useCallback(() => {
        if (!proxyUrl) return '';
        return proxyUrl.trimEnd().replace(/\/+$/, '').replace(/\/v1$/, '');
    }, [proxyUrl]);

    const checkStatus = useCallback(async () => {
        setLoading(true);
        try {
            const formattedUrl = getFormattedProxyUrl();
            const result = await invoke<OpenCodeStatus>('get_opencode_sync_status', {
                proxyUrl: formattedUrl
            });
            setStatus(result);
        } catch (error) {
            console.error('Failed to check OpenCode status:', error);
        } finally {
            setLoading(false);
        }
    }, [getFormattedProxyUrl]);

    const handleSync = () => {
        setSyncConfirmOpen(true);
    };

    const executeSync = async () => {
        setSyncConfirmOpen(false);

        if (!proxyUrl || !apiKey) {
            showToast(t('proxy.opencode_sync.toast.config_missing', { defaultValue: 'Please generate API Key and start the service first' }), 'error');
            return;
        }

        setSyncing(true);
        try {
            const formattedUrl = getFormattedProxyUrl();
            await invoke('execute_opencode_sync', {
                proxyUrl: formattedUrl,
                apiKey: apiKey,
                syncAccounts: syncAccounts
            });
            showToast(t('proxy.opencode_sync.toast.sync_success', { defaultValue: 'OpenCode synced successfully' }), 'success');
            await checkStatus();
        } catch (error: any) {
            showToast(t('proxy.opencode_sync.toast.sync_error', { defaultValue: `Sync failed: ${error.toString()}` }), 'error');
        } finally {
            setSyncing(false);
        }
    };

    const handleRestore = () => {
        setRestoreConfirmOpen(true);
    };

    const executeRestore = async () => {
        setRestoreConfirmOpen(false);

        setSyncing(true);
        try {
            await invoke('execute_opencode_restore');
            showToast(t('common.success', { defaultValue: 'Success' }), 'success');
            await checkStatus();
        } catch (error: any) {
            showToast(error.toString(), 'error');
        } finally {
            setSyncing(false);
        }
    };

    const handleViewConfig = async (fileName?: string) => {
        try {
            if (!status) return;

            const targetFile = fileName || status.files[0];
            const content = await invoke<string>('get_opencode_config_content', {
                fileName: targetFile
            });
            setViewingConfig({
                content,
                fileName: targetFile,
                allFiles: status.files
            });
        } catch (error: any) {
            showToast(error.toString(), 'error');
        }
    };

    useEffect(() => {
        checkStatus();
    }, [checkStatus]);

    const isAppLoading = loading;
    const isAppSyncing = syncing;

    return (
        <div className={cn("space-y-4", className)}>
            <div className="flex flex-col bg-white/50 dark:bg-gray-800/40 rounded-xl border border-gray-100 dark:border-white/5 p-4 shadow-sm hover:shadow-lg hover:border-blue-200/50 dark:hover:border-blue-500/30 transition-all duration-300 group">
                <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-y-3 gap-x-2 mb-4">
                    <div className="flex items-center gap-3 min-w-0">
                        <div className="p-2.5 bg-gray-50 dark:bg-base-300 rounded-lg shrink-0 group-hover:scale-110 transition-transform duration-300">
                            <Code size={20} className="text-blue-500" />
                        </div>
                        <div className="min-w-0">
                            <h4 className="text-sm font-bold text-gray-900 dark:text-gray-100 leading-tight truncate">
                                {t('proxy.opencode_sync.card_title', { defaultValue: 'OpenCode' })}
                            </h4>
                            <div className="mt-1 flex items-center gap-1.5 overflow-hidden">
                                {isAppLoading ? (
                                    <div className="flex items-center gap-1 text-[10px] text-gray-400">
                                        <Loader2 size={10} className="animate-spin" />
                                        {t('proxy.opencode_sync.status.detecting', { defaultValue: 'Detecting...' })}
                                    </div>
                                ) : status?.installed ? (
                                    <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 font-bold whitespace-nowrap">
                                        {t('proxy.opencode_sync.status.installed', { defaultValue: `v${status.version}`, version: status.version })}
                                    </span>
                                ) : (
                                    <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-gray-100 dark:bg-gray-800 text-gray-400 font-medium whitespace-nowrap">
                                        {t('proxy.opencode_sync.status.not_installed', { defaultValue: 'Not Installed' })}
                                    </span>
                                )}
                            </div>
                        </div>
                    </div>

                    {!isAppLoading && status?.installed && (
                        <div className={cn(
                            "inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-[10px] font-bold tracking-wide transition-all h-6 shrink-0 whitespace-nowrap shadow-sm",
                            status.is_synced
                                ? "bg-gradient-to-r from-green-500 to-emerald-600 text-white"
                                : "bg-amber-100 dark:bg-amber-900/30 text-amber-600 dark:text-amber-500 border border-amber-200/50 dark:border-amber-800/30"
                        )}>
                            {status.is_synced ? (
                                <><CheckCircle2 size={12} className="shrink-0" /> {t('proxy.opencode_sync.status.synced', { defaultValue: 'Synced' })}</>
                            ) : (
                                <><AlertCircle size={12} className="shrink-0" /> {t('proxy.opencode_sync.status.not_synced', { defaultValue: 'Not Synced' })}</>
                            )}
                        </div>
                    )}
                </div>

                <div className="mt-auto space-y-3">
                    <div className="p-2.5 bg-gray-50/80 dark:bg-gray-900/40 rounded-lg border border-dashed border-gray-200 dark:border-white/10">
                        <div className="flex justify-between items-start mb-1">
                            <div className="text-[9px] text-gray-400 dark:text-gray-500 uppercase font-bold tracking-wider">
                                {t('proxy.opencode_sync.status.current_base_url', { defaultValue: 'Current Base URL' })}
                            </div>
                        </div>
                        <div className="text-[10px] font-mono truncate text-gray-500 dark:text-gray-400 italic">
                            {status?.current_base_url || '---'}
                        </div>
                    </div>

                    {/* Sync Accounts Toggle */}
                    {status?.installed && (
                        <div className="flex items-center gap-2 p-2 bg-gray-50/50 dark:bg-gray-900/20 rounded-lg">
                            <input
                                type="checkbox"
                                id="opencode-sync-accounts"
                                checked={syncAccounts}
                                onChange={(e) => setSyncAccounts(e.target.checked)}
                                className="checkbox checkbox-xs checkbox-primary"
                            />
                            <label htmlFor="opencode-sync-accounts" className="text-[10px] text-gray-600 dark:text-gray-400 cursor-pointer select-none">
                                {t('proxy.opencode_sync.sync_accounts', { defaultValue: 'Sync accounts to antigravity-accounts.json' })}
                            </label>
                        </div>
                    )}

                    <div className="flex items-center gap-2">
                        {status?.installed && (
                            <>
                                <button
                                    onClick={() => handleViewConfig()}
                                    className="btn btn-sm btn-square btn-ghost border border-gray-200 dark:border-white/10 text-gray-500 hover:text-blue-500 hover:bg-white dark:hover:bg-gray-700"
                                    title={t('proxy.opencode_sync.btn_view', { defaultValue: 'View Config' })}
                                >
                                    <Eye size={16} />
                                </button>
                                <button
                                    onClick={handleRestore}
                                    className="btn btn-sm btn-square btn-ghost border border-gray-200 dark:border-white/10 text-gray-500 hover:text-orange-500 hover:bg-white dark:hover:bg-gray-700"
                                    title={status.has_backup ? t('proxy.opencode_sync.btn_restore_backup', { defaultValue: 'Restore from Backup' }) : t('proxy.opencode_sync.btn_restore', { defaultValue: 'Restore' })}
                                >
                                    <RotateCcw size={16} />
                                </button>
                            </>
                        )}
                        <button
                            onClick={handleSync}
                            disabled={!status?.installed || isAppSyncing || isAppLoading}
                            className={cn(
                                "btn btn-sm flex-1 gap-2 rounded-xl transition-all font-bold shadow-sm",
                                status?.is_synced
                                    ? "btn-ghost border-gray-200 dark:border-base-400 text-gray-500 hover:bg-gray-100"
                                    : "btn-primary hover:shadow-lg shadow-blue-500/20"
                            )}
                        >
                            {isAppSyncing ? (
                                <Loader2 size={14} className="animate-spin" />
                            ) : (
                                <RefreshCw size={14} className={cn(isAppLoading && "animate-spin-once")} />
                            )}
                            {t('proxy.opencode_sync.btn_sync', { defaultValue: 'Sync' })}
                        </button>
                    </div>
                </div>
            </div>

            {/* Config Viewer Modal */}
            {viewingConfig && (
                <div className="fixed inset-0 z-[300] flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm animate-in fade-in duration-200">
                    <div className="bg-white dark:bg-base-100 rounded-2xl shadow-2xl border border-gray-200 dark:border-base-300 w-full max-w-2xl overflow-hidden animate-in zoom-in-95 duration-200">
                        <div className="px-6 py-4 border-b border-gray-100 dark:border-base-200 flex items-center justify-between bg-gray-50/50 dark:bg-base-200/50">
                            <div>
                                <h3 className="font-bold text-gray-900 dark:text-base-content flex items-center gap-2">
                                    <Code size={18} className="text-blue-500" />
                                    {t('proxy.opencode_sync.modal.view_title', { defaultValue: 'OpenCode Config' })}
                                </h3>
                                <div className="mt-2 flex gap-2">
                                    {viewingConfig.allFiles.map(file => (
                                        <button
                                            key={file}
                                            onClick={() => handleViewConfig(file)}
                                            className={cn(
                                                "px-3 py-1 text-[10px] font-bold rounded-lg transition-all border",
                                                viewingConfig.fileName === file
                                                    ? "bg-blue-500 text-white border-blue-500"
                                                    : "bg-white dark:bg-base-300 text-gray-400 border-gray-100 dark:border-base-400 hover:border-blue-200"
                                            )}
                                        >
                                            {file}
                                        </button>
                                    ))}
                                </div>
                            </div>
                            <div className="flex items-center gap-2">
                                <button
                                    onClick={async () => {
                                        const success = await copyToClipboard(viewingConfig.content);
                                        if (success) {
                                            showToast(t('proxy.opencode_sync.modal.copy_success', { defaultValue: 'Copied!' }), 'success');
                                        }
                                    }}
                                    className="btn btn-ghost btn-sm hover:bg-blue-50 hover:text-blue-600 dark:hover:bg-blue-900/20"
                                >
                                    <Copy size={16} />
                                </button>
                                <button
                                    onClick={() => setViewingConfig(null)}
                                    className="btn btn-ghost btn-sm hover:bg-red-50 hover:text-red-600 dark:hover:bg-red-900/20"
                                >
                                    <X size={18} />
                                </button>
                            </div>
                        </div>
                        <div className="p-6">
                            <div className="bg-gray-900 rounded-xl p-4 overflow-auto max-h-[50vh] border border-gray-800 shadow-inner">
                                <pre className="text-xs font-mono text-gray-300 leading-relaxed">
                                    {viewingConfig.content}
                                </pre>
                            </div>
                        </div>
                    </div>
                </div>
            )}

            {/* Restore Confirmation Modal */}
            <ModalDialog
                isOpen={restoreConfirmOpen}
                title={status?.has_backup
                    ? t('proxy.opencode_sync.btn_restore_backup', { defaultValue: 'Restore from Backup' })
                    : t('proxy.opencode_sync.btn_restore', { defaultValue: 'Restore' })}
                message={status?.has_backup
                    ? t('proxy.opencode_sync.restore_backup_confirm', { defaultValue: 'Restore OpenCode configuration from backup?' })
                    : t('proxy.opencode_sync.restore_confirm', { defaultValue: 'Restore OpenCode to default configuration?' })}
                onConfirm={executeRestore}
                onCancel={() => setRestoreConfirmOpen(false)}
                isDestructive={true}
            />

            {/* Sync Confirmation Modal */}
            <ModalDialog
                isOpen={syncConfirmOpen}
                title={t('proxy.opencode_sync.sync_confirm_title', { defaultValue: 'Confirm Sync' })}
                message={t('proxy.opencode_sync.sync_confirm_message', { defaultValue: 'Sync OpenCode configuration with Antigravity proxy settings?' })}
                onConfirm={executeSync}
                onCancel={() => setSyncConfirmOpen(false)}
                isDestructive={true}
            />
        </div>
    );
};
