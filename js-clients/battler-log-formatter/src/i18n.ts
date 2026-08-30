import i18next from "i18next";
import { en } from "../locales/en.js";

i18next.init({
  lng: "en",
  fallbackLng: "en",
  returnNull: true,
  resources: {
    en: { translation: en }
  },
  interpolation: { escapeValue: false }
});

export default i18next;
