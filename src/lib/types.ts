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

export interface Library {
  roms_dir: string;
  installed: Record<string, Install>;
}

export interface RomStatus {
  ready: boolean;
  path: string | null;
  browse_dir: string;
}
