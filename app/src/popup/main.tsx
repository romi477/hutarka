import React from "react";
import ReactDOM from "react-dom/client";
import "@/styles.css";
import { Popup } from "./Popup";

// The panel window is transparent: only the card is drawn.
document.documentElement.style.background = "transparent";
document.body.style.background = "transparent";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Popup />
  </React.StrictMode>,
);
