import React from 'react';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'icon';
  size?: 'sm' | 'md' | 'lg';
}

export const Button: React.FC<ButtonProps> = ({ 
  children, 
  variant = 'primary', 
  size = 'md', 
  className = '', 
  ...props 
}) => {
  const baseClasses = "inline-flex items-center justify-center transition-all duration-300 ease-out focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-stone-900 disabled:opacity-50 disabled:cursor-not-allowed";
  
  const variants = {
    primary: "bg-stone-900 text-stone-50 hover:bg-stone-800 border border-transparent active:scale-[0.98]",
    secondary: "bg-white text-stone-900 border border-stone-200 hover:border-stone-400 hover:shadow-sm active:bg-stone-50",
    ghost: "bg-transparent text-stone-600 hover:text-stone-900 hover:bg-stone-100",
    icon: "bg-transparent text-stone-500 hover:text-stone-900 p-2 hover:bg-stone-100 rounded-full"
  };

  const sizes = {
    sm: "text-xs px-3 py-1.5",
    md: "text-sm px-5 py-2.5",
    lg: "text-base px-6 py-3"
  };

  // Override size for icon variant
  const sizeClass = variant === 'icon' ? '' : sizes[size];

  return (
    <button 
      className={`${baseClasses} ${variants[variant]} ${sizeClass} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};