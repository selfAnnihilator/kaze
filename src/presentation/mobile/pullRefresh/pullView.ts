import { resist, pullArmed } from "./pullMath";

/**
 * Drives the pull indicator straight through the DOM (no React state per
 * frame): it follows the finger, spins while a refresh runs, then springs back.
 */
export class PullView {
  constructor(private readonly el: HTMLElement) {}

  follow(dy: number) {
    const y = resist(dy);
    this.el.classList.add("is-pulling");
    this.el.classList.toggle("is-armed", pullArmed(dy));
    this.el.style.setProperty("--pull-y", `${y}px`);
    this.el.style.setProperty("--pull-turn", `${y * 4}deg`);
  }

  reset() {
    this.el.classList.remove("is-pulling", "is-armed", "is-spinning");
    this.el.style.setProperty("--pull-y", "0px");
  }

  /** Holds the indicator spinning until `work` settles. */
  async spin(work: Promise<unknown>) {
    this.el.classList.add("is-spinning");
    try {
      await work;
    } finally {
      this.reset();
    }
  }
}
