/**
 * Invoke a Tauri backend command from the frontend.
 * Enables communication between React and Rust (via Tauri IPC).
 * Only available when running in Tauri desktop environment (IS_TAURI = true).
 * @param {string} command - The Tauri command name to invoke
 * @param {object} args - Arguments to pass to the command
 * @returns {Promise<*>} Result from the Tauri backend
 * @throws {Error} If Tauri runtime is not available
 */
export const invokeTauri = async (command, args = {}) => {
  const tauriCore = await import("@tauri-apps/api/core");
  return tauriCore.invoke(command, args);
};
