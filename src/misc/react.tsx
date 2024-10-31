import "./main.css"

import React from "react";
import ReactDOM from "react-dom/client";
import { NextUIProvider } from '@nextui-org/react'

export function render(MainApp: () => JSX.Element) {
  return ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <NextUIProvider style={{width: "100%", height: "100%"}}>
        <main className="dark text-foreground bg-background">
          <MainApp />
        </main>
      </NextUIProvider>
    </React.StrictMode>,
  );
}
