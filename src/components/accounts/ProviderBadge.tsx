import { useTranslation } from 'react-i18next';
import type { Account } from '../../types/account';

/**
 * [FORK] Marks which upstream an account belongs to.
 *
 * Google accounts are the historical default and stay unlabelled so the pool does not
 * get noisier for existing users; only ChatGPT accounts get a badge.
 */
function ProviderBadge({ account }: { account: Account }) {
    const { t } = useTranslation();

    if ((account.provider ?? 'google') !== 'openai') {
        return null;
    }

    const plan = account.openai_plan || account.openai?.plan_type;

    return (
        <span
            className="px-2 py-0.5 rounded-md bg-emerald-100 dark:bg-emerald-900/50 text-emerald-700 dark:text-emerald-300 text-[10px] font-bold shadow-sm border border-emerald-200/50 dark:border-emerald-800/50"
            title={t(
                'accounts.provider.openai_tooltip',
                'ChatGPT account (Codex OAuth), served via the ChatGPT backend'
            )}
        >
            {plan ? `CHATGPT ${plan.toUpperCase()}` : 'CHATGPT'}
        </span>
    );
}

export default ProviderBadge;
