import React from 'react';
import { 
  BarChart, 
  Bar, 
  XAxis, 
  YAxis, 
  CartesianGrid, 
  Tooltip, 
  ResponsiveContainer, 
  Cell 
} from 'recharts';
import { FontMetrics } from '../types';

interface MetricsChartProps {
  metrics: FontMetrics;
}

export const MetricsChart: React.FC<MetricsChartProps> = ({ metrics }) => {
  const data = [
    { name: 'X-Height', value: metrics.xHeightRatio * 100, full: 100 },
    { name: 'Contrast', value: Math.min(metrics.strokeContrast * 10, 100), full: 100 },
    { name: 'Width Var', value: metrics.widthVariance * 200, full: 100 }, // Scale for visibility
    { name: 'Aperture', value: metrics.aperture * 100, full: 100 },
  ];

  return (
    <div className="w-full h-48 mt-6">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart
          layout="vertical"
          data={data}
          margin={{ top: 0, right: 30, left: 40, bottom: 0 }}
          barSize={12}
        >
          <CartesianGrid strokeDasharray="3 3" horizontal={false} stroke="#e5e5e5" />
          <XAxis type="number" hide domain={[0, 100]} />
          <YAxis 
            dataKey="name" 
            type="category" 
            tick={{ fontSize: 10, fill: '#71717a', fontFamily: 'sans-serif' }} 
            width={70}
            axisLine={false}
            tickLine={false}
          />
          <Tooltip 
            cursor={{fill: '#f4f4f5'}}
            contentStyle={{ 
              backgroundColor: '#fff', 
              border: '1px solid #e4e4e7', 
              boxShadow: '0 2px 8px rgba(0,0,0,0.05)',
              fontSize: '12px'
            }}
            formatter={(value: number) => [value.toFixed(1), 'Score']}
          />
          <Bar dataKey="value" radius={[0, 2, 2, 0]}>
            {data.map((entry, index) => (
              <Cell key={`cell-${index}`} fill="#3f3f46" />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
};
