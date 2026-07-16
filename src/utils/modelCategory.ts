/**
 * 模型分类工具函数（无 React / icons 依赖，可在 Node 环境直接导入）
 */

export type ModelCategory = 'gemini-pro' | 'gemini-flash' | 'gemini-pro-image' | 'gemini-flash-image' | 'claude' | 'other';

export function categorizeModel(name: string): ModelCategory {
    const n = name.trim().toLowerCase();
    const isGemini = n.startsWith('gemini-');
    const isImage = (isGemini && n.includes('image')) || n.startsWith('image') || n.startsWith('imagen');
    if (isImage) return n.includes('flash') ? 'gemini-flash-image' : 'gemini-pro-image';
    if (isGemini && n.includes('flash')) return 'gemini-flash';
    if (isGemini && n.includes('pro')) return 'gemini-pro';
    if (n.includes('claude') || n.includes('opus') || n.includes('sonnet') || n.includes('haiku')) return 'claude';
    return 'other';
}

export interface ModelDisplayNameInput {
    name: string;
    display_name?: string;
}

export function getModelDisplayName(
    model: ModelDisplayNameInput | null | undefined,
    fallback?: string,
): string {
    if (model) {
        if (model.display_name) return model.display_name;
        if (model.name) return model.name;
    }
    return fallback ?? '';
}

export function getModelProtectionKey(name: string): string | null {
    switch (categorizeModel(name)) {
        case 'gemini-flash': return 'gemini-3-flash';
        case 'gemini-pro': return 'gemini-3-pro-high';
        case 'gemini-flash-image': return 'gemini-3.1-flash-image';
        case 'gemini-pro-image': return 'gemini-3-pro-image';
        case 'claude': return 'claude';
        default: return null;
    }
}
