export default async (request, context) => {
  const password = Netlify.env.get("SITE_PASSWORD");
  if (!password || given(request) === password) return context.next();
  return new Response("This site asks for a password.", {
    status: 401,
    headers: { "WWW-Authenticate": 'Basic realm="teq.build", charset="UTF-8"', "Cache-Control": "no-store" },
  });
};

function given(request) {
  const [scheme, credentials] = (request.headers.get("authorization") || "").trim().split(/\s+/);
  if (!scheme || scheme.toLowerCase() !== "basic" || !credentials) return "";
  try {
    const bytes = Uint8Array.from(atob(credentials), (c) => c.charCodeAt(0));
    return new TextDecoder().decode(bytes).split(":").slice(1).join(":");
  } catch {
    return "";
  }
}

export const config = { path: "/*" };
