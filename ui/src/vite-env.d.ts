/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL: string;
  readonly TEST?: boolean;
  // add more env variables as needed
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
