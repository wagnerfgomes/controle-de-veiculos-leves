import React from "react";
import ReactDOM from "react-dom/client";

// As fontes entram no bundle como arquivo local. Nenhuma requisição de rede em
// runtime: as máquinas podem não ter internet, e fonte que não carrega quebra a
// densidade da tela inteira.
import "@fontsource/barlow/400.css";
import "@fontsource/barlow/500.css";
import "@fontsource/barlow/600.css";
import "@fontsource/barlow-condensed/500.css";
import "@fontsource/barlow-condensed/600.css";
import "@fontsource/barlow-condensed/700.css";

import { App } from "./App";
import "./estilo/global.css";
import "./estilo/layout.css";

const raiz = document.getElementById("raiz");
if (!raiz) throw new Error("elemento #raiz não encontrado em index.html");

ReactDOM.createRoot(raiz).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
