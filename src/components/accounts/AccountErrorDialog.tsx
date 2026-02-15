import { Ban, Lock, Clock, ExternalLink } from 'lucide-react';
import { Account } from '../../types/account';
import { formatDate } from '../../utils/format';
import { useTranslation } from 'react-i18next';
import ModalDialog from '../common/ModalDialog';

interface AccountErrorDialogProps {
    account: Account | null;
    onClose: () => void;
}

export default function AccountErrorDialog({ account, onClose }: AccountErrorDialogProps) {
    const { t } = useTranslation();
    if (!account) return null;

    const isForbidden = !!account.quota?.is_forbidden;
    const isDisabled = Boolean(account.disabled);
    const isProxyDisabled = account.proxy_disabled;

    const rawReason = account.disabled_reason || account.quota?.forbidden_reason || account.proxy_disabled_reason || '';

    // 解析错误消息，如果是 JSON 则提取 error.message
    const extractErrorMessage = (raw: string) => {
        const trimmed = raw.trim();
        if (!trimmed) return raw;
        try {
            const parsed = JSON.parse(trimmed);
            if (parsed?.error?.message) {
                return String(parsed.error.message);
            }
        } catch (_) {
            // 不处理
        }
        return raw;
    };

    const message = extractErrorMessage(rawReason);

    // 渲染带链接的文本
    const renderMessageWithLinks = (text: string) => {
        const urlRegex = /(https?:\/\/[^\s]+)/g;
        const parts = text.split(urlRegex);
        return parts.map((part, i) => {
            if (part.match(urlRegex)) {
                return (
                    <a
                        key={i}
                        href={part}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-blue-600 dark:text-blue-400 underline hover:text-blue-700 dark:hover:text-blue-300 break-all inline-flex items-center gap-1"
                        onClick={(e) => e.stopPropagation()}
                    >
                        {t('accounts.click_to_verify', '点击去验证')}
                        <ExternalLink className="w-3 h-3" />
                    </a>
                );
            }
            return part;
        });
    };

    return (
        <ModalDialog
            isOpen={true}
            title={t('accounts.error_details')}
            type="error"
            onConfirm={onClose}
            confirmText={t('common.close')}
        >
            <div className="space-y-5 py-2">
                {/* Account Info */}
                <div>
                    <label className="text-[10px] font-bold text-gray-400 dark:text-gray-500 uppercase tracking-wider block mb-1.5 ml-1">
                        {t('accounts.account')}
                    </label>
                    <div className="text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-50 dark:bg-base-200/50 px-4 py-2.5 rounded-xl border border-gray-100 dark:border-base-200 shadow-sm">
                        {account.email}
                    </div>
                </div>

                {/* Status */}
                <div>
                    <label className="text-[10px] font-bold text-gray-400 dark:text-gray-500 uppercase tracking-wider block mb-1.5 ml-1">
                        {t('accounts.error_status')}
                    </label>
                    <div className="flex flex-wrap gap-2">
                        {isForbidden && (
                            <span className="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400 text-xs font-bold ring-1 ring-red-200/50 dark:ring-red-900/20">
                                <Lock className="w-3 h-3" />
                                {t('accounts.status.forbidden')}
                            </span>
                        )}
                        {isDisabled && (
                            <span className="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-rose-100 text-rose-700 dark:bg-rose-900/30 dark:text-rose-400 text-xs font-bold ring-1 ring-rose-200/50 dark:ring-rose-900/20">
                                <Ban className="w-3 h-3" />
                                {t('accounts.status.disabled')}
                            </span>
                        )}
                        {isProxyDisabled && (
                            <span className="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400 text-xs font-bold ring-1 ring-orange-200/50 dark:ring-orange-900/20">
                                <Ban className="w-3 h-3" />
                                {t('accounts.status.proxy_disabled')}
                            </span>
                        )}
                    </div>
                </div>

                {/* Reason */}
                <div>
                    <label className="text-[10px] font-bold text-gray-400 dark:text-gray-500 uppercase tracking-wider block mb-1.5 ml-1">
                        {t('common.reason', '原因')}
                    </label>
                    <div className="text-xs text-red-600 dark:text-red-400 bg-red-50/50 dark:bg-red-900/10 p-4 rounded-xl border border-red-100 dark:border-red-900/20 break-all leading-relaxed font-mono shadow-inner min-h-[80px]">
                        {message ? renderMessageWithLinks(message) : t('common.unknown')}
                    </div>
                </div>

                {/* Time */}
                <div className="flex items-center gap-2 text-[11px] text-gray-400 dark:text-gray-500 pl-1">
                    <Clock size={12} strokeWidth={2.5} />
                    <span>
                        {t('accounts.error_time')}: {account.disabled_at ? formatDate(account.disabled_at) : (account.quota?.last_updated ? formatDate(account.quota.last_updated) : t('common.unknown'))}
                    </span>
                </div>
            </div>
        </ModalDialog>
    );
}
