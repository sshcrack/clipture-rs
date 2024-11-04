import "./main.css"

import { warn, debug, trace, info, error } from '@tauri-apps/plugin-log';
import React from "react";
import ReactDOM from "react-dom/client";
import { NextUIProvider } from '@nextui-org/react'


function forwardConsole(
  fnName: 'log' | 'debug' | 'info' | 'warn' | 'error',
  logger: (message: string) => Promise<void>
) {
  const original = console[fnName];
  console[fnName] = (message) => {
    original(message);
    if (typeof message === 'object') {
      logger(JSON.stringify(message));
    } else {
      logger(message);
    }
  };
}

//@ts-ignore
if (!window.consoleForwarded) {
  forwardConsole('log', trace);
  forwardConsole('debug', debug);
  forwardConsole('info', info);
  forwardConsole('warn', warn);
  forwardConsole('error', error);

  //@ts-ignore
  window.consoleForwarded = true;
}

export function render(MainApp: () => JSX.Element) {
  return ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <NextUIProvider style={{ width: "100%", height: "100%" }}>
        <main className="dark text-foreground bg-background">
          <MainApp />
        </main>
      </NextUIProvider>
    </React.StrictMode>,
  );
}
