export type ActionType = 'shortcut' | 'media' | 'app' | 'command' | 'link';
export type MetricType = 'ram_percent' | 'cpu_percent';

export interface MonitorConfig {
  metricType: MetricType;
  intervalMs: number;
}

export interface ButtonConfig {
  id: string;
  label: string;
  icon: string; // Replaces emoji with an icon (e.g., "mdi:play", "lucide:settings")
  emoji?: string; // Backward compatibility with old config
  backgroundColor: string;
  actionType: ActionType;
  buttonKind?: 'action' | 'monitor';
  monitorConfig?: MonitorConfig;
  // For 'shortcut': e.g. "Ctrl+Shift+Tab" or a standalone key like "Play"
  shortcutValue?: string;
  // For 'media': play_pause, volume_up, volume_down, mute, next, prev
  mediaAction?: string;
  // For 'app': path to the .exe file, e.g. "C:\\Windows\\notepad.exe"
  appPath?: string;
  // For 'command': raw shell string (sh -c or cmd /C), runs with user privileges
  commandValue?: string;
  // For 'link': http(s) URL — opens with the OS default browser (Windows `cmd /c start ""`, macOS `open`, Linux `xdg-open`)
  linkUrl?: string;
  // For custom icon rendering configuration (cover, contain, fill, normal)
  iconSizing?: 'normal' | 'cover' | 'contain' | 'fill';
  genshinFrame?: number;
}

export interface Page {
  id: string;
  name?: string;
  buttons: ButtonConfig[];
}

export interface Layout {
  rows: number;
  cols: number;
  buttons: ButtonConfig[];
  pages?: Page[];
  theme?: string;
}

export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'error';

export interface InstalledApp {
  name: string;
  path: string;
  icon?: string;
  publisher?: string;
}

export interface WSMessage {
  type: 'auth' | 'ping' | 'pong' | 'press' | 'sync_layout' | 'toast' | 'metric_update' | 'device_info';
  payload?: any;
}
