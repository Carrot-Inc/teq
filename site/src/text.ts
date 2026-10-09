// A lead's inline code, `like this`, rendered; it carries nothing else of Markdown.
export const inline = (text: string) =>
  text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/`([^`]+)`/g, "<code>$1</code>");
