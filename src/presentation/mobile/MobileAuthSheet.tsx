import { useState, type FormEvent } from "react";
import { LogIn, UserPlus, X } from "lucide-react";
import type { UserProfile } from "../../types";

type Mode = "login" | "signup";
type AuthFn = (username: string, password: string) => Promise<UserProfile | null>;

type Rule = [broken: (u: string, p: string, c: string) => boolean, message: string];
const ALWAYS: Rule[] = [
  [(u) => !u, "Enter your username."],
  [(_u, p) => !p, "Enter your password."],
];
/** The account server's own sign-up rules, checked here so mistakes show before a request. */
const SIGN_UP: Rule[] = [
  [(u) => u.length < 3 || u.length > 50, "Usernames are 3 to 50 characters."],
  [(u) => !/^[\p{L}\p{N}_.-]+$/u.test(u), "Use letters, numbers, dots, dashes or underscores."],
  [(_u, p) => p.length < 8 || p.length > 128, "Passwords are 8 to 128 characters."],
  [(_u, p, c) => p !== c, "The passwords don’t match."],
];

function validate(mode: Mode, username: string, password: string, confirm: string): string | null {
  const rules = mode === "login" ? ALWAYS : [...ALWAYS, ...SIGN_UP];
  return rules.find(([broken]) => broken(username, password, confirm))?.[1] ?? null;
}

function useAuthForm(onLogin: AuthFn, onSignUp: AuthFn, onClose: () => void) {
  const [mode, setMode] = useState<Mode>("login");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const switchMode = (next: Mode) => { setMode(next); setError(null); };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const name = username.trim();
    const problem = validate(mode, name, password, confirm);
    if (problem) return setError(problem);
    setBusy(true);
    setError(null);
    try {
      const profile = await (mode === "login" ? onLogin : onSignUp)(name, password);
      if (profile) onClose();
      else setError(mode === "login" ? "Couldn’t sign in. Check your username and password." : "Couldn’t create the account.");
    } catch (failure) {
      setError(typeof failure === "string" ? failure : (failure as Error)?.message || "Couldn’t reach the account server.");
    } finally {
      setBusy(false);
    }
  };
  return { mode, switchMode, username, setUsername, password, setPassword, confirm, setConfirm, error, busy, submit };
}

/**
 * Sign In / Create Account (Figma G6): a full-screen sheet with the two modes as
 * tabs, the account fields, the main action and "Continue as guest".
 */
export function MobileAuthSheet({ onClose, onLogin, onSignUp }: { onClose: () => void; onLogin: AuthFn; onSignUp: AuthFn }) {
  const f = useAuthForm(onLogin, onSignUp, onClose);
  const signup = f.mode === "signup";

  return (
    <section className="m-sheet m-auth" role="dialog" aria-modal="true" aria-label={signup ? "Create account" : "Sign in"}>
      <div className="m-backbar">
        <button type="button" className="m-backbar__back" onClick={onClose}>
          <span className="m-backbar__btn"><X size={20} strokeWidth={2} /></span>
          <span className="m-kicker m-kicker--ink">CLOSE</span>
        </button>
      </div>
      <form className="m-auth__form" onSubmit={(e) => void f.submit(e)} noValidate>
        <div className="m-auth__head">
          <span className="m-kicker">ACCOUNT // SYNC LIKES, PLAYLISTS &amp; STATS</span>
          <h1 className="m-display m-auth__title">{signup ? "CREATE ACCOUNT." : "SIGN IN."}</h1>
        </div>
        <div className="m-auth__tabs" role="tablist">
          <button type="button" role="tab" aria-selected={!signup} className={`m-auth__tab ${signup ? "" : "is-active"}`} onClick={() => f.switchMode("login")}>SIGN IN</button>
          <button type="button" role="tab" aria-selected={signup} className={`m-auth__tab ${signup ? "is-active" : ""}`} onClick={() => f.switchMode("signup")}>CREATE ACCOUNT</button>
        </div>
        <div className="m-auth__fields">
          <label className="m-auth__field">
            <span className="m-kicker m-kicker--ink">USERNAME</span>
            <input className="m-auth__input" value={f.username} onChange={(e) => f.setUsername(e.target.value)}
              autoComplete="username" autoCapitalize="none" autoCorrect="off" spellCheck={false} enterKeyHint="next" />
          </label>
          <label className="m-auth__field">
            <span className="m-kicker m-kicker--ink">PASSWORD</span>
            <input className="m-auth__input" type="password" value={f.password} onChange={(e) => f.setPassword(e.target.value)}
              autoComplete={signup ? "new-password" : "current-password"} enterKeyHint={signup ? "next" : "go"} />
          </label>
          <label className={`m-auth__field ${signup ? "" : "is-off"}`}>
            <span className="m-kicker m-kicker--ink">CONFIRM PASSWORD (NEW ACCOUNTS)</span>
            <input className="m-auth__input" type="password" value={f.confirm} onChange={(e) => f.setConfirm(e.target.value)}
              placeholder="Repeat password" disabled={!signup} autoComplete="new-password" enterKeyHint="go" />
          </label>
          {f.error && <p className="m-auth__error" role="alert">{f.error}</p>}
        </div>
        <div className="m-auth__actions">
          <button type="submit" className="m-btn m-auth__submit" disabled={f.busy}>
            {signup ? <UserPlus size={18} strokeWidth={2} /> : <LogIn size={18} strokeWidth={2} />}
            <span>{f.busy ? "PLEASE WAIT…" : signup ? "CREATE ACCOUNT" : "SIGN IN"}</span>
          </button>
          <button type="button" className="m-auth__guest" onClick={onClose}>CONTINUE AS GUEST</button>
        </div>
      </form>
    </section>
  );
}
