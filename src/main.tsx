import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./index.css";
import "./kaze-theme.css";

// Disable the native right-click context menu (Inspect Element, Reload, etc.)
document.addEventListener("contextmenu", (e) => e.preventDefault());

const rootElement = document.getElementById("root");

if (rootElement) {
  ReactDOM.createRoot(rootElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}
