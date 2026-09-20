import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.scss";
import { initPWA } from "./pwa";
import ErrorBoundary from "./ui/Common/ErrorBoundary";
import TypeChartStandaloneApp from "./ui/Standalone/TypeChartStandaloneApp";

// Register Service Worker for offline asset caching
initPWA();

const base = import.meta.env.BASE_URL || "/";
const baseNoTrailing = base.endsWith("/") ? base.slice(0, -1) : base;
const cleanPath = window.location.pathname.replace(baseNoTrailing, "") || "/";
const isStandaloneTypeChart = cleanPath === "/type-chart" || cleanPath === "/type-chart/";

const root = createRoot(document.getElementById("root")!);

if (isStandaloneTypeChart) {
  root.render(
    <StrictMode>
      <ErrorBoundary>
        <TypeChartStandaloneApp />
      </ErrorBoundary>
    </StrictMode>,
  );
} else {
  // Dynamically import MainApp so battle engine WASM, WAMP, and Redux are not loaded in the standalone type chart
  import("./MainApp").then(({ default: MainApp }) => {
    root.render(
      <StrictMode>
        <ErrorBoundary>
          <MainApp />
        </ErrorBoundary>
      </StrictMode>,
    );
  });
}
