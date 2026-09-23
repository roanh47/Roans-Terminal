import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface TermDataPayload {
  sessionId: string;
  data: number[];
}
interface TermExitPayload {
  sessionId: string;
  message?: string;
}

export class Session {
  readonly id: string;
  readonly term: Terminal;
  private readonly fit: FitAddon;
  private readonly unlisten: UnlistenFn[] = [];
  private cols = 80;
  private rows = 24;
  onExit?: (s: Session, message?: string) => void;

  constructor(id: string, el: HTMLElement, bg: string, fg: string) {
    this.id = id;
    this.term = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: 'Menlo, "DejaVu Sans Mono", "Fira Code", Consolas, monospace',
      scrollback: 5000,
      theme: { background: bg, foreground: fg },
    });
    this.fit = new FitAddon();
    this.term.loadAddon(this.fit);
    this.term.loadAddon(new WebLinksAddon());
    this.term.open(el);
    this.fit.fit();
    this.cols = this.term.cols;
    this.rows = this.term.rows;

    this.term.onData((d) => {
      void invoke("write", { sessionId: this.id, data: d });
    });
    this.term.onResize(({ cols, rows }) => {
      this.cols = cols;
      this.rows = rows;
      void invoke("resize", { sessionId: this.id, cols, rows });
    });

    void listen<TermDataPayload>("term-data", (e) => {
      if (e.payload.sessionId === this.id) {
        this.term.write(new Uint8Array(e.payload.data));
      }
    }).then((u) => this.unlisten.push(u));

    void listen<TermExitPayload>("term-exit", (e) => {
      if (e.payload.sessionId === this.id) {
        this.onExit?.(this, e.payload.message);
      }
    }).then((u) => this.unlisten.push(u));
  }

  setTheme(bg: string, fg: string) {
    this.term.options.theme = { background: bg, foreground: fg };
  }

  syncResize() {
    this.cols = this.term.cols;
    this.rows = this.term.rows;
    void invoke("resize", { sessionId: this.id, cols: this.cols, rows: this.rows });
  }

  refit() {
    this.fit.fit();
  }

  dispose() {
    this.unlisten.forEach((u) => u());
    this.term.dispose();
  }
}
