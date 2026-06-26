import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Toaster } from "@/components/ui/sonner";

document.documentElement.classList.add("dark");

if (import.meta.env.PROD) {
  document.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <TooltipProvider delayDuration={200}>
      <App />
      <Toaster position="bottom-right" />
    </TooltipProvider>
  </React.StrictMode>,
);
