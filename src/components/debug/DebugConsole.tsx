import React, { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { motion, AnimatePresence } from 'framer-motion';
import { Terminal, X, Trash2, Search, ArrowDownToLine, Pause, Play, Bug, Info, AlertTriangle, AlertOctagon } from 'lucide-react';
import { useDebugConsole, LogEntry, LogLevel } from '../../stores/useDebugConsole';
import { cn } from '../../utils/cn';

const LEVEL_CONFIG: Record<LogLevel, { color: string, icon: React.ReactNode, label: string }> = {
    'ERROR': { color: 'text-red-500', icon: <AlertOctagon size={12} />, label: 'Error' },
    'WARN': { color: 'text-amber-500', icon: <AlertTriangle size={12} />, label: 'Warn' },
    'INFO': { color: 'text-blue-500', icon: <Info size={12} />, label: 'Info' },
    'DEBUG': { color: 'text-zinc-400', icon: <Bug size={12} />, label: 'Debug' },
    'TRACE': { color: 'text-zinc-600', icon: <Terminal size={12} />, label: 'Trace' },
};

const LogRow = React.memo(({ log }: { log: LogEntry }) => {
    const [expanded, setExpanded] = useState(false);
    const date = new Date(log.timestamp);
    const timeStr = date.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' }) + '.' + date.getMilliseconds().toString().padStart(3, '0');

    const hasFields = Object.keys(log.fields).length > 0;

    return (
        <div className="font-mono text-[11px] border-b border-white/5 hover:bg-white/5 transition-colors">
            <div
                className={cn("flex gap-2 px-2 py-1 items-start cursor-default", hasFields && "cursor-pointer hover:bg-white/10")}
                onClick={() => hasFields && setExpanded(!expanded)}
            >
                <span className="text-zinc-500 shrink-0 select-none min-w-[85px]">{timeStr}</span>
                <span className={cn("shrink-0 min-w-[50px] font-bold uppercase flex items-center gap-1", LEVEL_CONFIG[log.level as LogLevel].color)}>
                    {LEVEL_CONFIG[log.level as LogLevel].icon}
                    {log.level}
                </span>
                <span className="text-zinc-400 shrink-0 min-w-[120px] max-w-[120px] truncate" title={log.target}>
                    {log.target.split('::').slice(-2).join('::')}
                </span>
                <span className={cn("flex-1 break-words whitespace-pre-wrap", LEVEL_CONFIG[log.level as LogLevel].color)}>
                    {log.message}
                </span>
            </div>

            {expanded && hasFields && (
                <div className="px-4 py-2 bg-black/20 text-zinc-400 border-t border-white/5">
                    <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
                        {Object.entries(log.fields).map(([key, value]) => (
                            <React.Fragment key={key}>
                                <span className="text-zinc-500 text-right">{key}:</span>
                                <span className="text-zinc-300 break-all select-text">{value}</span>
                            </React.Fragment>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
});

const DebugConsole: React.FC = () => {
    const { t } = useTranslation();
    const {
        isOpen, close, logs, clearLogs,
        filter, setFilter,
        searchTerm, setSearchTerm,
        autoScroll, setAutoScroll,
        checkEnabled
    } = useDebugConsole();

    const scrollRef = useRef<HTMLDivElement>(null);
    const [height, setHeight] = useState(320);

    // Initial check
    useEffect(() => {
        checkEnabled();
    }, []);

    // Auto scroll
    useEffect(() => {
        if (autoScroll && scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
    }, [logs, autoScroll, isOpen]);

    // Handle resize
    const startResizing = (e: React.MouseEvent) => {
        e.preventDefault();
        document.addEventListener('mousemove', handleMouseMove);
        document.addEventListener('mouseup', stopResizing);
    };

    const handleMouseMove = (e: MouseEvent) => {
        const newHeight = window.innerHeight - e.clientY;
        if (newHeight > 100 && newHeight < window.innerHeight - 100) {
            setHeight(newHeight);
        }
    };

    const stopResizing = () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', stopResizing);
    };

    const toggleLevel = (level: LogLevel) => {
        if (filter.includes(level)) {
            setFilter(filter.filter(l => l !== level));
        } else {
            setFilter([...filter, level]);
        }
    };

    const scrollToBottom = () => {
        if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
            setAutoScroll(true);
        }
    };

    const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
        const element = e.currentTarget;
        const isAtBottom = Math.abs(element.scrollHeight - element.scrollTop - element.clientHeight) < 20;
        if (!isAtBottom && autoScroll) {
            setAutoScroll(false);
        } else if (isAtBottom && !autoScroll) {
            setAutoScroll(true);
        }
    };

    const filteredLogs = logs.filter(log => {
        if (!filter.includes(log.level as LogLevel)) return false;
        if (searchTerm && !log.message.toLowerCase().includes(searchTerm.toLowerCase()) &&
            !log.target.toLowerCase().includes(searchTerm.toLowerCase())) return false;
        return true;
    });

    return (
        <AnimatePresence>
            {isOpen && (
                <>
                    {/* Backdrop */}
                    <motion.div
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                        className="fixed inset-0 bg-black/10 z-[9998]"
                        onClick={close}
                    />

                    {/* Console Panel */}
                    <motion.div
                        initial={{ y: "100%" }}
                        animate={{ y: 0 }}
                        exit={{ y: "100%" }}
                        transition={{ type: "spring", stiffness: 300, damping: 30 }}
                        style={{ height }}
                        className="fixed bottom-0 left-0 right-0 bg-[#0c0c0c] border-t border-zinc-800 shadow-2xl z-[9999] flex flex-col font-sans"
                    >
                        {/* Resize Handle */}
                        <div
                            className="h-1 bg-zinc-800 hover:bg-blue-500 cursor-ns-resize transition-colors w-full"
                            onMouseDown={startResizing}
                        />

                        {/* Toolbar */}
                        <div className="flex items-center justify-between px-2 py-1.5 bg-zinc-900 border-b border-zinc-800 select-none">
                            <div className="flex items-center gap-2">
                                <span className="flex items-center gap-2 text-zinc-400 font-medium px-2 text-xs">
                                    <Terminal size={14} />
                                    Debug Console
                                </span>
                                <div className="h-4 w-px bg-zinc-800 mx-1" />

                                {/* Filter Toggles */}
                                <div className="flex bg-zinc-950 rounded p-0.5 border border-zinc-800">
                                    {(Object.keys(LEVEL_CONFIG) as LogLevel[]).map(level => (
                                        <button
                                            key={level}
                                            onClick={() => toggleLevel(level)}
                                            className={cn(
                                                "px-2 py-0.5 text-[10px] rounded transition-colors",
                                                filter.includes(level)
                                                    ? LEVEL_CONFIG[level].color + " bg-white/5"
                                                    : "text-zinc-600 hover:text-zinc-500"
                                            )}
                                        >
                                            {level}
                                        </button>
                                    ))}
                                </div>

                                {/* Search */}
                                <div className="relative group">
                                    <Search size={12} className="absolute left-2 top-1.5 text-zinc-500 group-focus-within:text-zinc-300" />
                                    <input
                                        type="text"
                                        value={searchTerm}
                                        onChange={e => setSearchTerm(e.target.value)}
                                        placeholder="Filter logs..."
                                        className="bg-zinc-950 border border-zinc-800 rounded pl-7 pr-2 py-0.5 text-xs text-zinc-300 w-32 focus:w-48 transition-all focus:outline-none focus:border-zinc-700 placeholder:text-zinc-700"
                                    />
                                </div>
                            </div>

                            <div className="flex items-center gap-1">
                                <button
                                    onClick={() => setAutoScroll(!autoScroll)}
                                    className={cn(
                                        "p-1.5 rounded transition-colors",
                                        autoScroll ? "text-green-400 bg-green-500/10" : "text-zinc-500 hover:text-zinc-300"
                                    )}
                                    title={autoScroll ? t('debug_console.pause_scroll', { defaultValue: 'Pause scroll' }) : t('debug_console.resume_scroll', { defaultValue: 'Resume scroll' })}
                                >
                                    {autoScroll ? <Pause size={14} /> : <Play size={14} />}
                                </button>

                                <button
                                    onClick={clearLogs}
                                    className="p-1.5 rounded bg-zinc-800 text-zinc-400 hover:text-red-400 hover:bg-zinc-700 transition-colors"
                                    title={t('debug_console.clear', { defaultValue: 'Clear' })}
                                >
                                    <Trash2 size={14} />
                                </button>

                                <button
                                    onClick={close}
                                    className="p-1.5 rounded bg-zinc-800 text-zinc-400 hover:text-white hover:bg-zinc-700 transition-colors"
                                >
                                    <X size={14} />
                                </button>
                            </div>
                        </div>

                        {/* Log content */}
                        <div
                            ref={scrollRef}
                            onScroll={handleScroll}
                            className="flex-1 overflow-y-auto overflow-x-hidden bg-zinc-950"
                        >
                            {filteredLogs.length === 0 ? (
                                <div className="flex flex-col items-center justify-center h-full text-zinc-600">
                                    <Terminal size={48} className="mb-4 opacity-50" />
                                    <p className="text-sm">{t('debug_console.no_logs', { defaultValue: 'No logs to display' })}</p>
                                    <p className="text-xs mt-1">{t('debug_console.no_logs_hint', { defaultValue: 'Logs will appear here in real-time' })}</p>
                                </div>
                            ) : (
                                filteredLogs.map(log => <LogRow key={log.id} log={log} />)
                            )}
                        </div>

                        {/* Footer */}
                        <div className="flex items-center justify-between px-4 py-2 border-t border-white/10 bg-zinc-800/50">
                            <div className="flex items-center gap-3">
                                {/* Level stats */}
                                {(Object.keys(LEVEL_CONFIG) as LogLevel[]).map(level => {
                                    const count = logs.filter(l => l.level === level).length;
                                    if (count === 0) return null;
                                    return (
                                        <span
                                            key={level}
                                            className={cn("text-[10px] font-mono flex items-center gap-1", LEVEL_CONFIG[level].color)}
                                        >
                                            {LEVEL_CONFIG[level].icon}
                                            {count}
                                        </span>
                                    );
                                })}
                            </div>

                            {/* Auto-scroll indicator */}
                            {!autoScroll && (
                                <button
                                    onClick={scrollToBottom}
                                    className="flex items-center gap-1 px-2 py-1 rounded bg-blue-500/20 text-blue-400 text-xs font-medium hover:bg-blue-500/30 transition-colors"
                                >
                                    <ArrowDownToLine size={12} />
                                    {t('debug_console.scroll_to_bottom', { defaultValue: 'Scroll to bottom' })}
                                </button>
                            )}
                        </div>
                    </motion.div>
                </>
            )}
        </AnimatePresence>
    );
};

export default DebugConsole;
