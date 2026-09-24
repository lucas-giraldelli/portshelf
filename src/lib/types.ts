export type Media = "cartridge" | "disc";
export type Kind = "recomp" | "decomp" | "build";

export interface Console {
  name: string;
  media: Media;
  color: string;
  year: number;
}

export interface Port {
  id: string;
  name: string;
  console: string;
  kind: Kind;
  repo: string;
  config?: { format: string; dir?: string };
  game_arg?: string;
  /** No-Intro / Redump title, used to find cover art. */
  title?: string;
  /** Cartridge colour when it is not the console default (DK64 yellow, Zelda gold). */
  shell?: string;
}

export interface Catalog {
  version: number;
  consoles: Record<string, Console>;
  ports: Port[];
}

export interface Install {
  exec: string;
  args: string[];
  cwd?: string;
  config_dir?: string;
  cover?: string;
}

export interface Override {
  name?: string;
  cover_locked?: boolean;
}

export interface Library {
  roms_dir: string;
  installed: Record<string, Install>;
  overrides: Record<string, Override>;
}

export interface RomStatus {
  ready: boolean;
  path: string | null;
  browse_dir: string;
}
