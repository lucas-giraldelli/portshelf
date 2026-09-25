export type Media = "cartridge" | "disc";
export type Kind = "recomp" | "decomp" | "build" | "remake" | "custom";

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
  /** False for a port that has not been released yet: listed, but it cannot be installed. */
  available?: boolean;
  /** Operating systems the port exists for, when not all of them. */
  platforms?: string[];
  /** The release of the game the port needs, and the hashes ("algo:hex") of the accepted files. */
  rom?: { needs: string; hashes?: string[] };
  /** Who made the port; defaults to the GitHub owner. */
  authors?: string;
  /** How PortShelf installs it, per operating system. */
  install?: Partial<Record<"linux" | "windows" | "macos", { asset: string[]; exec?: string }>>;
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
  /** How the port gets its game file; absent when the port asks for it itself. */
  rom?: { kind: string } | null;
  version?: string;
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
  /** Release of a file in the game's folder that the port does not accept. */
  wrong: string | null;
}
