import { useEffect, useRef, useState } from 'react';
import { KeyRound, Loader2, Link2, Copy, Check, ClipboardPaste, Info } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
    getOpenAiAuthLink,
    importOpenAiAuthJson,
    pollOpenAiDeviceLogin,
    startOpenAiDeviceLogin,
    submitOpenAiCode,
    type OpenAiDeviceCode,
} from '../../services/accountService';
import { copyToClipboard } from '../../utils/clipboard';

/**
 * [FORK] ChatGPT (OpenAI) login.
 *
 * Three ways in, all usable on a headless server:
 * - device code: open a URL, type a short code (recommended, nothing to paste back);
 * - link + code: open the authorization URL, paste the callback URL back;
 * - auth.json: paste the credentials of a Codex CLI that is already signed in.
 */
type Mode = 'device' | 'link' | 'json';

interface Props {
    /** Called after an account was added so the caller can refresh the pool. */
    onAdded: (email: string) => void;
}

function ChatGptLoginPanel({ onAdded }: Props) {
    const { t } = useTranslation();
    const [mode, setMode] = useState<Mode>('device');
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState('');
    const [info, setInfo] = useState('');

    const [device, setDevice] = useState<OpenAiDeviceCode | null>(null);
    const [codeCopied, setCodeCopied] = useState(false);

    const [authUrl, setAuthUrl] = useState('');
    const [authUrlCopied, setAuthUrlCopied] = useState(false);
    const [pastedCode, setPastedCode] = useState('');

    const [authJson, setAuthJson] = useState('');

    // Device polling must stop when the panel unmounts or the mode changes.
    const pollTimer = useRef<number | null>(null);
    const stopPolling = () => {
        if (pollTimer.current !== null) {
            window.clearTimeout(pollTimer.current);
            pollTimer.current = null;
        }
    };

    useEffect(() => stopPolling, []);
    useEffect(() => {
        stopPolling();
        setError('');
        setInfo('');
    }, [mode]);

    const fail = (e: unknown) => {
        setError(typeof e === 'string' ? e : (e as Error)?.message || String(e));
        setBusy(false);
    };

    const succeed = (email: string) => {
        stopPolling();
        setBusy(false);
        setDevice(null);
        setAuthUrl('');
        setPastedCode('');
        setAuthJson('');
        setInfo(
            t('accounts.add.chatgpt.added', 'ChatGPT account added: {{email}}', { email })
        );
        onAdded(email);
    };

    const beginDeviceLogin = async () => {
        setBusy(true);
        setError('');
        setInfo('');
        try {
            const started = await startOpenAiDeviceLogin();
            setDevice(started);
            setInfo(
                t(
                    'accounts.add.chatgpt.device_hint',
                    'Open the link, sign in to ChatGPT and enter the code. This window keeps checking.'
                )
            );
            schedulePoll(started, started.interval || 5);
        } catch (e) {
            fail(e);
        }
    };

    const schedulePoll = (started: OpenAiDeviceCode, intervalSeconds: number) => {
        stopPolling();
        pollTimer.current = window.setTimeout(async () => {
            try {
                const result = await pollOpenAiDeviceLogin(started.device_auth_id);
                if (result.status === 'complete' && result.account) {
                    succeed(result.account.email);
                    return;
                }
                if (Date.now() / 1000 > started.expires_at) {
                    stopPolling();
                    setBusy(false);
                    setDevice(null);
                    setError(
                        t('accounts.add.chatgpt.device_expired', 'The code expired, start again')
                    );
                    return;
                }
                schedulePoll(started, result.interval || intervalSeconds);
            } catch (e) {
                setDevice(null);
                fail(e);
            }
        }, Math.max(1, intervalSeconds) * 1000);
    };

    const requestAuthLink = async () => {
        setBusy(true);
        setError('');
        setInfo('');
        try {
            const link = await getOpenAiAuthLink();
            setAuthUrl(link.url);
            setInfo(
                t(
                    'accounts.add.chatgpt.link_hint',
                    'Open the link and sign in. The browser will land on a localhost page that cannot load — copy that whole address here.'
                )
            );
        } catch (e) {
            fail(e);
        } finally {
            setBusy(false);
        }
    };

    const submitCode = async () => {
        if (!pastedCode.trim()) return;
        setBusy(true);
        setError('');
        try {
            const result = await submitOpenAiCode(pastedCode.trim());
            succeed(result.account.email);
        } catch (e) {
            fail(e);
        }
    };

    const submitAuthJson = async () => {
        if (!authJson.trim()) return;
        setBusy(true);
        setError('');
        try {
            const result = await importOpenAiAuthJson(authJson);
            succeed(result.account.email);
        } catch (e) {
            fail(e);
        }
    };

    const copy = async (value: string, mark: (v: boolean) => void) => {
        await copyToClipboard(value);
        mark(true);
        window.setTimeout(() => mark(false), 1500);
    };

    const modeButton = (value: Mode, label: string) => (
        <button
            type="button"
            className={`py-1.5 px-3 rounded-lg text-xs font-medium transition-all ${
                mode === value
                    ? 'bg-white dark:bg-base-100 shadow-sm text-emerald-600 dark:text-emerald-400'
                    : 'text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
            }`}
            onClick={() => setMode(value)}
            disabled={busy}
        >
            {label}
        </button>
    );

    return (
        <div className="space-y-4 py-2">
            <div className="text-center space-y-2">
                <div className="bg-emerald-50 dark:bg-emerald-900/20 p-5 rounded-full w-16 h-16 mx-auto flex items-center justify-center">
                    <KeyRound className="w-8 h-8 text-emerald-500" />
                </div>
                <p className="text-sm text-gray-500 dark:text-gray-400 max-w-sm mx-auto">
                    {t(
                        'accounts.add.chatgpt.desc',
                        'Sign in with a ChatGPT subscription account (the same login Codex CLI uses). No API key needed.'
                    )}
                </p>
            </div>

            <div className="bg-gray-100 dark:bg-base-200 p-1 rounded-xl grid grid-cols-3 gap-1">
                {modeButton('device', t('accounts.add.chatgpt.mode_device', 'Device code'))}
                {modeButton('link', t('accounts.add.chatgpt.mode_link', 'Link + code'))}
                {modeButton('json', t('accounts.add.chatgpt.mode_json', 'auth.json'))}
            </div>

            {info && (
                <div className="text-xs flex items-start gap-2 text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-900/10 rounded-lg p-2">
                    <Info className="w-4 h-4 shrink-0 mt-0.5" />
                    <span>{info}</span>
                </div>
            )}
            {error && (
                <div className="text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/10 rounded-lg p-2 break-all">
                    {error}
                </div>
            )}

            {mode === 'device' && (
                <div className="space-y-3">
                    {!device && (
                        <button
                            className="btn btn-primary w-full"
                            onClick={beginDeviceLogin}
                            disabled={busy}
                        >
                            {busy ? (
                                <Loader2 className="w-4 h-4 animate-spin" />
                            ) : (
                                <KeyRound className="w-4 h-4" />
                            )}
                            {t('accounts.add.chatgpt.btn_device', 'Get a code')}
                        </button>
                    )}

                    {device && (
                        <div className="space-y-3">
                            <div className="flex items-center gap-2">
                                <input
                                    className="input input-bordered input-sm flex-1 font-mono text-xs"
                                    readOnly
                                    value={device.verification_url}
                                />
                                <a
                                    className="btn btn-sm btn-ghost"
                                    href={device.verification_url}
                                    target="_blank"
                                    rel="noreferrer"
                                >
                                    <Link2 className="w-4 h-4" />
                                </a>
                            </div>
                            <div className="flex items-center gap-2">
                                <div className="flex-1 text-center font-mono text-2xl tracking-widest bg-gray-50 dark:bg-base-200 rounded-lg py-3">
                                    {device.user_code}
                                </div>
                                <button
                                    className="btn btn-sm btn-ghost"
                                    onClick={() => copy(device.user_code, setCodeCopied)}
                                >
                                    {codeCopied ? (
                                        <Check className="w-4 h-4 text-green-500" />
                                    ) : (
                                        <Copy className="w-4 h-4" />
                                    )}
                                </button>
                            </div>
                            <div className="flex items-center justify-center gap-2 text-xs text-gray-500">
                                <Loader2 className="w-3 h-3 animate-spin" />
                                {t('accounts.add.chatgpt.waiting', 'Waiting for approval…')}
                            </div>
                        </div>
                    )}
                </div>
            )}

            {mode === 'link' && (
                <div className="space-y-3">
                    <button
                        className="btn btn-outline btn-sm w-full"
                        onClick={requestAuthLink}
                        disabled={busy}
                    >
                        {busy ? (
                            <Loader2 className="w-4 h-4 animate-spin" />
                        ) : (
                            <Link2 className="w-4 h-4" />
                        )}
                        {t('accounts.add.chatgpt.btn_link', 'Get the authorization link')}
                    </button>

                    {authUrl && (
                        <div className="flex items-center gap-2">
                            <input
                                className="input input-bordered input-sm flex-1 font-mono text-xs"
                                readOnly
                                value={authUrl}
                            />
                            <button
                                className="btn btn-sm btn-ghost"
                                onClick={() => copy(authUrl, setAuthUrlCopied)}
                            >
                                {authUrlCopied ? (
                                    <Check className="w-4 h-4 text-green-500" />
                                ) : (
                                    <Copy className="w-4 h-4" />
                                )}
                            </button>
                        </div>
                    )}

                    <textarea
                        className="textarea textarea-bordered w-full text-xs font-mono h-20"
                        placeholder={t(
                            'accounts.add.chatgpt.code_placeholder',
                            'http://localhost:1455/auth/callback?code=...'
                        )}
                        value={pastedCode}
                        onChange={(e) => setPastedCode(e.target.value)}
                        disabled={busy}
                    />
                    <button
                        className="btn btn-primary btn-sm w-full"
                        onClick={submitCode}
                        disabled={busy || !pastedCode.trim()}
                    >
                        {busy ? (
                            <Loader2 className="w-4 h-4 animate-spin" />
                        ) : (
                            <ClipboardPaste className="w-4 h-4" />
                        )}
                        {t('accounts.add.chatgpt.btn_submit_code', 'Submit code')}
                    </button>
                </div>
            )}

            {mode === 'json' && (
                <div className="space-y-3">
                    <p className="text-xs text-gray-500 dark:text-gray-400">
                        {t(
                            'accounts.add.chatgpt.json_hint',
                            'Paste the contents of ~/.codex/auth.json from a machine where Codex CLI is signed in.'
                        )}
                    </p>
                    <textarea
                        className="textarea textarea-bordered w-full text-xs font-mono h-32"
                        placeholder='{"tokens":{"id_token":"...","access_token":"...","refresh_token":"..."}}'
                        value={authJson}
                        onChange={(e) => setAuthJson(e.target.value)}
                        disabled={busy}
                    />
                    <button
                        className="btn btn-primary btn-sm w-full"
                        onClick={submitAuthJson}
                        disabled={busy || !authJson.trim()}
                    >
                        {busy ? (
                            <Loader2 className="w-4 h-4 animate-spin" />
                        ) : (
                            <ClipboardPaste className="w-4 h-4" />
                        )}
                        {t('accounts.add.chatgpt.btn_import_json', 'Import account')}
                    </button>
                </div>
            )}
        </div>
    );
}

export default ChatGptLoginPanel;
