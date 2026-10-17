// Vite's `?raw` imports, used by tests to read files from the Rust side.
declare module "*?raw" {
  const content: string;
  export default content;
}
