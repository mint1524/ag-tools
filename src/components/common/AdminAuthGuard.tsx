import React, { useState, useEffect } from 'react';
import { Lock, Key } from 'lucide-react';
import { isTauri } from '../../utils/env';

/**
 * AdminAuthGuard
 * 针对 Docker/Web 模式的强制鉴权保护层。
 * 如果检测到没有存储的 API Key 或后端返回 401，将拦截 UI 并要求输入 Key。
 */
export const AdminAuthGuard: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [isAuthenticated, setIsAuthenticated] = useState(isTauri());
    const [apiKey, setApiKey] = useState('');

    useEffect(() => {
        if (isTauri()) return;

        // 检查本地存储
        const savedKey = localStorage.getItem('abv_admin_api_key');
        if (savedKey) {
            setIsAuthenticated(true);
        }

        // 监听全局 401 事件（后续可以在 request.ts 中触发）
        const handleUnauthorized = () => {
            localStorage.removeItem('abv_admin_api_key');
            setIsAuthenticated(false);
        };

        window.addEventListener('abv-unauthorized', handleUnauthorized);
        return () => window.removeEventListener('abv-unauthorized', handleUnauthorized);
    }, []);

    const handleLogin = (e: React.FormEvent) => {
        e.preventDefault();
        if (apiKey.trim()) {
            localStorage.setItem('abv_admin_api_key', apiKey.trim());
            setIsAuthenticated(true);
            // 刷新页面以确保所有状态重新加载
            window.location.reload();
        }
    };

    if (isAuthenticated) {
        return <>{children}</>;
    }

    return (
        <div className="min-h-screen bg-slate-50 dark:bg-base-300 flex items-center justify-center p-4">
            <div className="max-w-md w-full bg-white dark:bg-base-100 rounded-3xl shadow-xl overflow-hidden border border-slate-100 dark:border-white/5">
                <div className="p-8">
                    <div className="w-16 h-16 bg-blue-50 dark:bg-blue-900/20 rounded-2xl flex items-center justify-center mb-6 mx-auto">
                        <Lock className="w-8 h-8 text-blue-500" />
                    </div>
                    <h2 className="text-2xl font-bold text-center text-slate-900 dark:text-slate-100 mb-2 font-display">安全访问控制</h2>
                    <p className="text-center text-slate-500 dark:text-slate-400 mb-8 text-sm">当前运行在 Web 模式下，请输入管理密码或 API Key 以进入后台。</p>

                    <form onSubmit={handleLogin} className="space-y-6">
                        <div className="relative">
                            <Key className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-slate-400" />
                            <input
                                type="password"
                                placeholder="请输入管理密码或 API Key"
                                className="w-full pl-12 pr-4 py-4 bg-slate-50 dark:bg-base-200 border-none rounded-2xl focus:ring-2 focus:ring-blue-500 transition-all outline-none text-slate-900 dark:text-white"
                                value={apiKey}
                                onChange={(e) => setApiKey(e.target.value)}
                                autoFocus
                            />
                        </div>
                        <button
                            type="submit"
                            className="w-full py-4 bg-blue-500 hover:bg-blue-600 text-white font-bold rounded-2xl shadow-lg shadow-blue-500/30 transition-all active:scale-[0.98]"
                        >
                            验证并进入
                        </button>
                    </form>

                    <div className="mt-8 pt-6 border-t border-slate-50 dark:border-white/5 text-center">
                        <p className="text-[10px] text-slate-400 leading-relaxed">
                            注意：如果设置了独立的管理密码，请输入管理密码；否则请输入 <code className="bg-slate-100 dark:bg-white/10 px-1 rounded">API_KEY</code>。
                            <br />
                            如果您忘记了，请运行 <code className="bg-slate-100 dark:bg-white/10 px-1 rounded">docker logs antigravity-manager</code> 寻找 <code className="text-blue-500">Current API Key</code> 或 <code className="text-blue-500">Web UI Password</code>
                            <br />
                            或执行 <code className="bg-slate-100 dark:bg-white/10 px-1 rounded">grep -E '"api_key"|"admin_password"' ~/.antigravity_tools/gui_config.json</code> 查看。
                        </p>
                    </div>
                </div>
            </div>
        </div>
    );
};
