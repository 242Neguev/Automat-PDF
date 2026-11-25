import React from "react";
import ReactDOM from "react-dom/client";
import { MantineProvider } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import "@mantine/core/styles.css";
import "@mantine/notifications/styles.css";
import "./interface.css";
import App from "./App";
import { MAC } from "./systeme";
import { suivreOrdinateur, theme, variables } from "./theme";

// Mac : fenêtre transparente, bureau flouté derrière (essai du 25/11/2026, tauri.macos.conf.json).
if (MAC) document.documentElement.dataset.fenetre = "verre";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <MantineProvider theme={theme} cssVariablesResolver={variables} defaultColorScheme="auto" colorSchemeManager={suivreOrdinateur}>
      <Notifications position="bottom-right" />
      <App />
    </MantineProvider>
  </React.StrictMode>,
);
