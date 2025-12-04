// Theme store with system preference detection and localStorage persistence

export type Theme = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';

const STORAGE_KEY = 'gat-demo-theme';

// Get the system preference
function getSystemTheme(): ResolvedTheme {
  if (typeof window === 'undefined') return 'dark';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

// Load saved preference from localStorage
function loadSavedTheme(): Theme {
  if (typeof window === 'undefined') return 'system';
  const saved = localStorage.getItem(STORAGE_KEY);
  if (saved === 'light' || saved === 'dark' || saved === 'system') {
    return saved;
  }
  return 'system';
}

// Resolve the actual theme to apply
function resolveTheme(theme: Theme): ResolvedTheme {
  if (theme === 'system') {
    return getSystemTheme();
  }
  return theme;
}

// Create the theme state
class ThemeState {
  preference = $state<Theme>(loadSavedTheme());
  resolved = $state<ResolvedTheme>(resolveTheme(loadSavedTheme()));

  constructor() {
    // Listen for system theme changes
    if (typeof window !== 'undefined') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      mediaQuery.addEventListener('change', () => {
        if (this.preference === 'system') {
          this.resolved = getSystemTheme();
        }
      });
    }
  }

  setTheme(theme: Theme) {
    this.preference = theme;
    this.resolved = resolveTheme(theme);
    if (typeof window !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, theme);
    }
  }

  toggle() {
    // Cycle through: system -> light -> dark -> system
    if (this.preference === 'system') {
      this.setTheme('light');
    } else if (this.preference === 'light') {
      this.setTheme('dark');
    } else {
      this.setTheme('system');
    }
  }

  // Quick toggle between light and dark (ignoring system)
  toggleLightDark() {
    this.setTheme(this.resolved === 'dark' ? 'light' : 'dark');
  }
}

export const themeState = new ThemeState();
