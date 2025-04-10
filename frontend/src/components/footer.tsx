"use client";

import React from 'react';
import Link from 'next/link';
import { motion } from 'framer-motion';
import { Github, Twitter, Linkedin, Heart } from 'lucide-react';

const Footer: React.FC = () => {
  const currentYear = new Date().getFullYear();
  
  const socialLinks = [
    { name: 'GitHub', icon: <Github size={16} />, href: 'https://github.com/roxom' },
    { name: 'Twitter', icon: <Twitter size={16} />, href: 'https://twitter.com/roxom' },
    { name: 'LinkedIn', icon: <Linkedin size={16} />, href: 'https://linkedin.com/in/roxom' },
  ];
  
  const quickLinks = [
    { name: 'About', href: '/about' },
    { name: 'Privacy', href: '/privacy' },
    { name: 'Terms', href: '/terms' },
    { name: 'FAQ', href: '/faq' },
    { name: 'Support', href: '/support' },
  ];

  return (
    <div className="w-full border-t border-border/40 bg-background/80 py-2">
      <div className="container mx-auto px-6">
        <div className="flex flex-row items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="h-5 w-5 rounded-full bg-gradient-to-tr from-green-600 to-green-400 flex items-center justify-center mr-1">
              <span className="text-white font-bold text-[10px]">$</span>
            </div>
            <span className="text-xs font-bold tracking-tight">Roxom</span>
            
            <div className="flex items-center gap-3 ml-4">
              {socialLinks.map((link) => (
                <Link 
                  key={link.name} 
                  href={link.href}
                  className="text-muted-foreground hover:text-foreground transition-colors"
                  aria-label={link.name}
                >
                  {link.icon}
                </Link>
              ))}
            </div>
          </div>
          
          <div className="flex items-center gap-4 text-xs text-muted-foreground">
            {quickLinks.map((link) => (
              <Link 
                key={link.name}
                href={link.href}
                className="hover:text-foreground transition-colors"
              >
                {link.name}
              </Link>
            ))}
            
            <div className="flex items-center gap-1 text-[10px] text-muted-foreground ml-4 border-l border-border/40 pl-4">
              <span>Made with</span>
              <Heart size={10} className="text-red-500" />
              <span>by Roxom Team</span>
              <span className="ml-2">© {currentYear}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Footer; 