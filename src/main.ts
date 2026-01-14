import { createApp } from "vue";
import App from "./App.vue";
import "./assets/main.css";

function syncColorScheme() {
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)");
  const apply = () => {
    document.documentElement.classList.toggle("dark", prefersDark.matches);
  };
  apply();
  prefersDark.addEventListener("change", apply);
}

if (typeof window !== "undefined") {
  syncColorScheme();
}

createApp(App).mount("#app");
