/**
 * Font Map Visualization
 * ========================
 * 
 * Interactive 2D scatter plot showing fonts by similarity.
 * Uses UMAP coordinates from Python analysis.
 */

import React, { useRef, useEffect, useState, useCallback } from 'react';

export interface FontMapNode {
    id: string;
    name: string;
    x: number;  // UMAP x coordinate (0-100)
    y: number;  // UMAP y coordinate (0-100)
    category: string;
    filePath?: string;
}

interface FontMapProps {
    fonts: FontMapNode[];
    width?: number;
    height?: number;
    onFontSelect?: (font: FontMapNode) => void;
    selectedFontId?: string | null;
    className?: string;
}

// Category colors
const CATEGORY_COLORS: Record<string, string> = {
    'Serif': '#e63946',
    'Sans-Serif': '#457b9d',
    'Display': '#f4a261',
    'Script': '#2a9d8f',
    'Monospace': '#9b5de5',
    'Unknown': '#888888',
};

export const FontMap: React.FC<FontMapProps> = ({
    fonts,
    width = 800,
    height = 600,
    onFontSelect,
    selectedFontId,
    className = ""
}) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [hoveredFont, setHoveredFont] = useState<FontMapNode | null>(null);
    const [transform, setTransform] = useState({ x: 0, y: 0, scale: 1 });
    const [isDragging, setIsDragging] = useState(false);
    const [dragStart, setDragStart] = useState({ x: 0, y: 0 });

    // Convert UMAP coords (0-100) to canvas coords
    const toCanvasCoords = useCallback((node: FontMapNode) => {
        const padding = 50;
        const x = padding + (node.x / 100) * (width - padding * 2);
        const y = padding + (node.y / 100) * (height - padding * 2);
        return {
            x: (x + transform.x) * transform.scale,
            y: (y + transform.y) * transform.scale
        };
    }, [width, height, transform]);

    // Draw the map
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        // Clear
        ctx.fillStyle = '#0a0a0f';
        ctx.fillRect(0, 0, width, height);

        // Draw grid
        ctx.strokeStyle = '#1a1a2e';
        ctx.lineWidth = 1;
        for (let i = 0; i <= 10; i++) {
            const x = (i / 10) * width;
            const y = (i / 10) * height;
            ctx.beginPath();
            ctx.moveTo(x, 0);
            ctx.lineTo(x, height);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(0, y);
            ctx.lineTo(width, y);
            ctx.stroke();
        }

        // Draw connections between similar fonts (optional)
        // Could add lines between nearby fonts

        // Draw font nodes
        fonts.forEach(font => {
            const { x, y } = toCanvasCoords(font);
            const color = CATEGORY_COLORS[font.category] || CATEGORY_COLORS['Unknown'];
            const isSelected = font.id === selectedFontId;
            const isHovered = font.id === hoveredFont?.id;

            const radius = isSelected ? 12 : isHovered ? 10 : 6;

            // Glow effect for selected/hovered
            if (isSelected || isHovered) {
                ctx.shadowBlur = 20;
                ctx.shadowColor = color;
            }

            // Draw node
            ctx.beginPath();
            ctx.arc(x, y, radius, 0, Math.PI * 2);
            ctx.fillStyle = color;
            ctx.fill();

            // Border
            ctx.strokeStyle = isSelected ? '#fff' : 'rgba(255,255,255,0.3)';
            ctx.lineWidth = isSelected ? 2 : 1;
            ctx.stroke();

            // Reset shadow
            ctx.shadowBlur = 0;
        });

        // Draw label for hovered font
        if (hoveredFont) {
            const { x, y } = toCanvasCoords(hoveredFont);

            ctx.font = '14px system-ui, sans-serif';
            ctx.fillStyle = '#fff';
            ctx.textAlign = 'center';
            ctx.fillText(hoveredFont.name, x, y - 20);
        }

        // Draw legend
        const legendX = width - 150;
        let legendY = 30;
        ctx.font = '12px system-ui, sans-serif';
        Object.entries(CATEGORY_COLORS).forEach(([category, color]) => {
            if (category === 'Unknown') return;

            ctx.beginPath();
            ctx.arc(legendX, legendY, 5, 0, Math.PI * 2);
            ctx.fillStyle = color;
            ctx.fill();

            ctx.fillStyle = '#888';
            ctx.textAlign = 'left';
            ctx.fillText(category, legendX + 15, legendY + 4);

            legendY += 20;
        });

    }, [fonts, width, height, toCanvasCoords, selectedFontId, hoveredFont]);

    // Mouse handlers
    const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const rect = canvas.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;

        if (isDragging) {
            setTransform(prev => ({
                ...prev,
                x: prev.x + (mouseX - dragStart.x) / prev.scale,
                y: prev.y + (mouseY - dragStart.y) / prev.scale
            }));
            setDragStart({ x: mouseX, y: mouseY });
            return;
        }

        // Find hovered font
        let found: FontMapNode | null = null;
        for (const font of fonts) {
            const { x, y } = toCanvasCoords(font);
            const dist = Math.sqrt((mouseX - x) ** 2 + (mouseY - y) ** 2);
            if (dist < 15) {
                found = font;
                break;
            }
        }
        setHoveredFont(found);
        canvas.style.cursor = found ? 'pointer' : 'grab';
    };

    const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
        if (hoveredFont) {
            onFontSelect?.(hoveredFont);
        } else {
            setIsDragging(true);
            setDragStart({ x: e.clientX, y: e.clientY });
        }
    };

    const handleMouseUp = () => {
        setIsDragging(false);
    };

    const handleWheel = (e: React.WheelEvent<HTMLCanvasElement>) => {
        e.preventDefault();
        const scaleDelta = e.deltaY > 0 ? 0.9 : 1.1;
        setTransform(prev => ({
            ...prev,
            scale: Math.max(0.5, Math.min(3, prev.scale * scaleDelta))
        }));
    };

    return (
        <div className={`font-map ${className}`} style={{ position: 'relative' }}>
            <canvas
                ref={canvasRef}
                width={width}
                height={height}
                onMouseMove={handleMouseMove}
                onMouseDown={handleMouseDown}
                onMouseUp={handleMouseUp}
                onMouseLeave={handleMouseUp}
                onWheel={handleWheel}
                style={{
                    borderRadius: 8,
                    cursor: isDragging ? 'grabbing' : 'grab'
                }}
            />

            {/* Zoom controls */}
            <div style={{
                position: 'absolute',
                bottom: 16,
                left: 16,
                display: 'flex',
                gap: 8
            }}>
                <button
                    onClick={() => setTransform(prev => ({ ...prev, scale: prev.scale * 1.2 }))}
                    style={{
                        width: 32,
                        height: 32,
                        borderRadius: 4,
                        border: 'none',
                        background: 'rgba(255,255,255,0.1)',
                        color: '#fff',
                        cursor: 'pointer',
                        fontSize: 18
                    }}
                >
                    +
                </button>
                <button
                    onClick={() => setTransform(prev => ({ ...prev, scale: prev.scale * 0.8 }))}
                    style={{
                        width: 32,
                        height: 32,
                        borderRadius: 4,
                        border: 'none',
                        background: 'rgba(255,255,255,0.1)',
                        color: '#fff',
                        cursor: 'pointer',
                        fontSize: 18
                    }}
                >
                    −
                </button>
                <button
                    onClick={() => setTransform({ x: 0, y: 0, scale: 1 })}
                    style={{
                        height: 32,
                        padding: '0 12px',
                        borderRadius: 4,
                        border: 'none',
                        background: 'rgba(255,255,255,0.1)',
                        color: '#fff',
                        cursor: 'pointer',
                        fontSize: 12
                    }}
                >
                    Reset
                </button>
            </div>

            {/* Font count */}
            <div style={{
                position: 'absolute',
                top: 16,
                left: 16,
                color: '#888',
                fontSize: 12
            }}>
                {fonts.length} fonts mapped
            </div>
        </div>
    );
};

/**
 * Generate mock UMAP coordinates for fonts
 * In production, these would come from Python UMAP analysis
 */
export function generateMockMapData(fonts: Array<{
    id: string;
    name: string;
    category: string;
    filePath?: string;
}>): FontMapNode[] {
    // Simple clustering by category
    const categoryOffsets: Record<string, { x: number; y: number }> = {
        'Serif': { x: 25, y: 25 },
        'Sans-Serif': { x: 75, y: 25 },
        'Display': { x: 25, y: 75 },
        'Script': { x: 75, y: 75 },
        'Monospace': { x: 50, y: 50 },
    };

    return fonts.map((font, i) => {
        const offset = categoryOffsets[font.category] || { x: 50, y: 50 };
        // Add some randomness within category cluster
        const jitter = 15;
        return {
            ...font,
            x: offset.x + (Math.random() - 0.5) * jitter * 2,
            y: offset.y + (Math.random() - 0.5) * jitter * 2,
        };
    });
}

export default FontMap;
