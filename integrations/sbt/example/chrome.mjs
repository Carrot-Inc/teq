// Headless Chrome for the browser checks: CHROME names the binary, else the platform's usual place.
export const chromePath =
  process.env.CHROME ?? (process.platform === "darwin" ? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" : "/usr/bin/google-chrome")
