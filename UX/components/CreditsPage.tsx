/**
 * Credits Page Component
 * ========================
 * 
 * About page with creator info and attributions
 */

import React from 'react';
import { X, Github, Twitter, Globe, Heart, Sparkles, Code, Cpu, Database, ExternalLink } from 'lucide-react';

interface CreditsPageProps {
    onClose: () => void;
}

const TECH_STACK = [
    { name: 'Tauri', icon: '🦀', description: 'Desktop app framework', url: 'https://tauri.app' },
    { name: 'React', icon: '⚛️', description: 'UI library', url: 'https://react.dev' },
    { name: 'Rust', icon: '🔧', description: 'Backend & performance', url: 'https://rust-lang.org' },
    { name: 'Python', icon: '🐍', description: 'Font analysis sidecar', url: 'https://python.org' },
    { name: 'SQLite', icon: '💾', description: 'Local database', url: 'https://sqlite.org' },
    { name: 'Qdrant', icon: '🔍', description: 'Vector database', url: 'https://qdrant.tech' },
];

const OSS_ATTRIBUTIONS = [
    { name: 'fontTools', license: 'MIT', description: 'Font file parsing', url: 'https://github.com/fonttools/fonttools' },
    { name: 'UMAP', license: 'BSD-3', description: 'Dimensionality reduction', url: 'https://umap-learn.readthedocs.io' },
    { name: 'rusqlite', license: 'MIT', description: 'SQLite bindings for Rust', url: 'https://github.com/rusqlite/rusqlite' },
    { name: 'Lucide Icons', license: 'ISC', description: 'Beautiful open source icons', url: 'https://lucide.dev' },
    { name: 'Tailwind CSS', license: 'MIT', description: 'Utility-first CSS framework', url: 'https://tailwindcss.com' },
    { name: 'Vite', license: 'MIT', description: 'Frontend tooling', url: 'https://vitejs.dev' },
    { name: 'chrono', license: 'MIT', description: 'Date/time for Rust', url: 'https://github.com/chronotope/chrono' },
    { name: 'uuid', license: 'MIT', description: 'UUID generation', url: 'https://github.com/uuid-rs/uuid' },
];

const AI_PLATFORMS = [
    { name: 'Gemini', description: 'AI assistant', provider: 'Google', url: 'https://gemini.google.com' },
    { name: 'Claude', description: 'Code generation', provider: 'Anthropic', url: 'https://claude.ai' },
];

export const CreditsPage: React.FC<CreditsPageProps> = ({ onClose }) => {
    return (
        <>
            {/* Backdrop */}
            <div
                className="fixed inset-0 bg-black/30 backdrop-blur-sm z-40"
                onClick={onClose}
            />

            {/* Modal - smaller and centered */}
            <div className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-lg bg-white rounded-2xl shadow-2xl z-50 overflow-hidden flex flex-col max-h-[85vh]">
                {/* Header */}
                <div className="flex items-center justify-between px-6 py-4 border-b border-stone-200 bg-stone-50">
                    <div className="flex items-center gap-3">
                        <div className="w-10 h-10 bg-gradient-to-br from-amber-400 to-orange-500 rounded-xl flex items-center justify-center">
                            <Sparkles size={20} className="text-white" />
                        </div>
                        <div>
                            <h1 className="text-lg font-bold text-stone-900">FontAtlas</h1>
                            <p className="text-xs text-stone-500">Version 0.1.0</p>
                        </div>
                    </div>
                    <button
                        onClick={onClose}
                        className="p-2 text-stone-400 hover:text-stone-600 hover:bg-stone-200 rounded-lg transition-colors"
                    >
                        <X size={20} />
                    </button>
                </div>

                {/* Content */}
                <div className="flex-1 overflow-y-auto p-6">
                    <div className="space-y-6">

                        {/* Creator Section */}
                        <section className="text-center pb-4 border-b border-stone-100">
                            <div className="w-16 h-16 mx-auto mb-3 rounded-full bg-gradient-to-br from-stone-800 to-stone-900 flex items-center justify-center">
                                <span className="text-2xl">👨‍💻</span>
                            </div>
                            <h2 className="text-lg font-semibold text-stone-900 mb-0.5">
                                Created by Viet X
                            </h2>
                            <p className="text-sm text-stone-500 mb-3">
                                Designer & Developer
                            </p>
                            <div className="flex items-center justify-center gap-2">
                                <a
                                    href="https://github.com/vietx"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="p-2 text-stone-500 hover:text-stone-900 hover:bg-stone-100 rounded-lg transition-colors"
                                >
                                    <Github size={18} />
                                </a>
                                <a
                                    href="https://twitter.com/vietx"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="p-2 text-stone-500 hover:text-stone-900 hover:bg-stone-100 rounded-lg transition-colors"
                                >
                                    <Twitter size={18} />
                                </a>
                                <a
                                    href="https://vietx.com"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="p-2 text-stone-500 hover:text-stone-900 hover:bg-stone-100 rounded-lg transition-colors"
                                >
                                    <Globe size={18} />
                                </a>
                            </div>
                        </section>

                        {/* Tech Stack */}
                        <section>
                            <h3 className="flex items-center gap-2 text-xs font-semibold text-stone-400 uppercase tracking-wider mb-3">
                                <Code size={14} />
                                Built With
                            </h3>
                            <div className="grid grid-cols-2 gap-2">
                                {TECH_STACK.map(tech => (
                                    <a
                                        key={tech.name}
                                        href={tech.url}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        className="flex items-center gap-2.5 p-2.5 bg-stone-50 hover:bg-stone-100 rounded-lg transition-colors group"
                                    >
                                        <span className="text-lg">{tech.icon}</span>
                                        <div className="flex-1 min-w-0">
                                            <div className="text-sm font-medium text-stone-900 group-hover:text-amber-600 transition-colors">{tech.name}</div>
                                            <div className="text-[10px] text-stone-400 truncate">{tech.description}</div>
                                        </div>
                                        <ExternalLink size={12} className="text-stone-300 group-hover:text-amber-500 transition-colors" />
                                    </a>
                                ))}
                            </div>
                        </section>

                        {/* AI Platforms */}
                        <section>
                            <h3 className="flex items-center gap-2 text-xs font-semibold text-stone-400 uppercase tracking-wider mb-3">
                                <Cpu size={14} />
                                AI-Assisted
                            </h3>
                            <div className="flex gap-2">
                                {AI_PLATFORMS.map(platform => (
                                    <a
                                        key={platform.name}
                                        href={platform.url}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        className="flex-1 flex items-center justify-between px-3 py-2 bg-gradient-to-br from-purple-50 to-blue-50 hover:from-purple-100 hover:to-blue-100 rounded-lg transition-colors group"
                                    >
                                        <div>
                                            <div className="text-sm font-medium text-stone-900">{platform.name}</div>
                                            <div className="text-[10px] text-stone-500">{platform.provider}</div>
                                        </div>
                                        <ExternalLink size={12} className="text-stone-300 group-hover:text-purple-500 transition-colors" />
                                    </a>
                                ))}
                            </div>
                        </section>

                        {/* Open Source */}
                        <section>
                            <h3 className="flex items-center gap-2 text-xs font-semibold text-stone-400 uppercase tracking-wider mb-3">
                                <Database size={14} />
                                Open Source Libraries
                            </h3>
                            <div className="bg-stone-50 rounded-lg divide-y divide-stone-100">
                                {OSS_ATTRIBUTIONS.map(lib => (
                                    <a
                                        key={lib.name}
                                        href={lib.url}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        className="flex items-center justify-between px-3 py-2 hover:bg-stone-100 transition-colors group first:rounded-t-lg last:rounded-b-lg"
                                    >
                                        <div className="flex items-center gap-2">
                                            <span className="text-sm font-medium text-stone-800 group-hover:text-amber-600 transition-colors">{lib.name}</span>
                                            <span className="text-[10px] text-stone-400">{lib.description}</span>
                                        </div>
                                        <div className="flex items-center gap-2">
                                            <span className="text-[10px] px-1.5 py-0.5 bg-stone-200 text-stone-500 rounded">
                                                {lib.license}
                                            </span>
                                            <ExternalLink size={10} className="text-stone-300 group-hover:text-amber-500 transition-colors" />
                                        </div>
                                    </a>
                                ))}
                            </div>
                        </section>
                    </div>
                </div>

                {/* Footer */}
                <div className="px-6 py-3 border-t border-stone-100 bg-stone-50 text-center">
                    <p className="flex items-center justify-center gap-1.5 text-xs text-stone-500">
                        Made with <Heart size={12} className="text-red-500 fill-red-500" /> for typography lovers
                    </p>
                    <p className="text-[10px] text-stone-400 mt-0.5">
                        © 2025 Viet X. All rights reserved.
                    </p>
                </div>
            </div>
        </>
    );
};

export default CreditsPage;
