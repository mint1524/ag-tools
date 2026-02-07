import { useTranslation } from "react-i18next";
import { GlobalSystemPromptConfig } from "../../types/config";

interface GlobalSystemPromptProps {
    config: GlobalSystemPromptConfig;
    onChange: (config: GlobalSystemPromptConfig) => void;
}

const DEFAULT_CONFIG: GlobalSystemPromptConfig = {
    enabled: false,
    content: '',
};

export default function GlobalSystemPrompt({
    config = DEFAULT_CONFIG,
    onChange,
}: GlobalSystemPromptProps) {
    const { t } = useTranslation();

    return (
        <div className="space-y-4">
            {/* 标题区域 */}
            <div className="bg-purple-50/50 dark:bg-purple-900/10 border border-purple-100 dark:border-purple-800/30 rounded-lg p-4">
                <div className="flex items-center justify-between">
                    <div className="space-y-1 flex-1">
                        <h4 className="font-medium text-sm text-gray-900 dark:text-gray-100">
                            {t("settings.global_system_prompt.title", { defaultValue: "全局系统提示词 (Global System Prompt)" })}
                        </h4>
                        <p className="text-xs text-gray-500 dark:text-gray-400 leading-relaxed">
                            {t("settings.global_system_prompt.description", {
                                defaultValue: "设置一段全局提示词，将自动注入到所有 API 请求的 systemInstruction 中，位于 Antigravity 身份之后、客户端提示词之前。",
                            })}
                        </p>
                    </div>
                    {/* 启用/禁用开关 */}
                    <label className="relative inline-flex items-center cursor-pointer ml-4 shrink-0">
                        <input
                            type="checkbox"
                            checked={config.enabled}
                            onChange={(e) => onChange({ ...config, enabled: e.target.checked })}
                            className="sr-only peer"
                        />
                        <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-purple-300 dark:peer-focus:ring-purple-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:after:border-gray-600 peer-checked:bg-purple-600"></div>
                    </label>
                </div>
            </div>

            {/* 编辑区域 (仅在启用时显示) */}
            {config.enabled && (
                <div className="space-y-3">
                    <textarea
                        value={config.content}
                        onChange={(e) => onChange({ ...config, content: e.target.value })}
                        placeholder={t("settings.global_system_prompt.placeholder", {
                            defaultValue: "输入全局系统提示词...\n例如：你是一位资深的全栈开发工程师，擅长 React 和 Rust。请使用简体中文回复。",
                        })}
                        rows={6}
                        className="w-full bg-white dark:bg-base-100 border border-gray-200 dark:border-gray-700 rounded-lg px-4 py-3 text-sm focus:ring-2 focus:ring-purple-500/20 outline-none transition-all resize-y min-h-[120px]"
                    />
                    <div className="flex items-center justify-between">
                        <p className="text-xs text-gray-400 dark:text-gray-500">
                            {t("settings.global_system_prompt.char_count", {
                                defaultValue: "{{count}} 字符",
                                count: config.content.length,
                            })}
                        </p>
                    </div>
                    {config.content.length > 2000 && (
                        <div className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-700/30 rounded-lg p-3">
                            <p className="text-xs text-amber-700 dark:text-amber-400">
                                {t("settings.global_system_prompt.long_prompt_warning", {
                                    defaultValue: "提示词较长（超过 2000 字符），可能会占用较多的上下文窗口空间，影响模型可用的对话长度。",
                                })}
                            </p>
                        </div>
                    )}
                </div>
            )}
        </div>
    );
}
