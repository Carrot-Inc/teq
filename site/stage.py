#!/usr/bin/env python3
"""site/stage.py: stages the site's content from the repository's Markdown, so that the site has no text of its own.

README.md is split at its `## ` headings into fragments the front page composes (src/pages/index.astro names the
order): the intro's first paragraph is the hero's lead and each paragraph after it a point of the strip under the
hero, titled by its bold opening, and the
compilation-speed section's first paragraph stands beside its heading, the rest of its text under the cards drawn
from its table (.staged/speed.json). The documents the README's "Documentation" table names become the pages under
/docs/, in the table's order, each titled by its first heading. A backticked `docs/X.md` in any of them becomes a link
to that page where the page exists, and a code fence whose first line is a comment naming a file gets that name as
its caption. The name, the tagline, the headline (the README's centred h1, its lines kept apart), the lead, the
plugin's version and the opening words of the status (.staged/site.json) are what the hero and the page titles read.
Everything lands under site/.staged/ (git-ignored), and the logo is copied from assets/logo/ into site/public/assets/
(git-ignored too), beside the Markdown files for language models (/llms.txt, /llms-full.txt, /index.md, /docs/*.md).
`npm run build` runs this first.
"""
import json
import os
import re
import shutil
import sys

site = os.path.dirname(os.path.abspath(__file__))
root = os.path.dirname(site)
staged = os.path.join(site, ".staged")
public = os.path.join(site, "public")
with open(os.path.join(site, "astro.config.mjs"), encoding="utf-8") as f:
    site_url = re.search(r'site: "([^"]+)"', f.read()).group(1).rstrip("/")


def read(path):
    with open(os.path.join(root, path), encoding="utf-8") as f:
        return f.read()


written = set()


def write(path, text):
    """Writes a staged file only when its text changed, so that a dev server watching .staged/ sees an edit as an
    edit; the directory is never emptied and rebuilt, which its content store does not follow."""
    written.add(os.path.abspath(path))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    try:
        with open(path, encoding="utf-8") as f:
            if f.read() == text:
                return
    except FileNotFoundError:
        pass
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def front(fields):
    return "---\n" + "".join(f"{k}: {json.dumps(v)}\n" for k, v in fields.items()) + "---\n\n"


readme = read("README.md")

# The documents to publish: every `docs/...` path of the README's "Documentation" table, in the table's order, with
# the row's description. A directory lists its .md files.
docs = []
table = re.search(r"^## Documentation\n(.*?)(?=^## |\Z)", readme, re.S | re.M).group(1)
for line in table.splitlines():
    cells = [c.strip() for c in line.strip().strip("|").split("|")]
    if len(cells) != 2 or not (cells[0].startswith("`") or cells[0].startswith("[")):
        continue
    for path in dict.fromkeys(re.findall(r"docs/[A-Za-z0-9_./-]+", cells[0])):
        if path.endswith("/"):
            for name in sorted(os.listdir(os.path.join(root, path))):
                if name.endswith(".md"):
                    docs.append((path + name, cells[1]))
        else:
            docs.append((path, cells[1]))


def slug_of(path):
    return re.sub(r"[^a-z0-9]+", "-", path[len("docs/"):-len(".md")].lower()).strip("-")


def heading_slug(title):
    """A heading's anchor as GitHub makes it: lowercased, punctuation dropped, spaces as hyphens."""
    return re.sub(r" ", "-", re.sub(r"[^\w\s-]", "", title.strip().lower()))


slugs = {path: slug_of(path) for path, _ in docs}


def link_docs(text):
    """`docs/X.md` and [..](docs/X.md) become links to the rendered page; an unpublished path stays as it is."""
    return link_paths(caption_fences(tag_fences(markdown_safe(text))), lambda slug: f"/docs/{slug}/")


def link_raw(text):
    """The same links for the Markdown files served to language models, to the files themselves."""
    return link_paths(text, lambda slug: f"{site_url}/docs/{slug}.md")


def link_paths(text, url):
    def code(m):
        path = m.group(1)
        return f"[`{path}`]({url(slugs[path])})" if path in slugs else m.group(0)
    text = re.sub(r"(?<!\]\()`(docs/[A-Za-z0-9_./-]+\.md)`", code, text)

    def target(m):
        path = m.group(1)
        return f"]({url(slugs[path])})" if path in slugs else m.group(0)
    return re.sub(r"\]\((docs/[A-Za-z0-9_./-]+\.md)\)", target, text)


def markdown_safe(text):
    """The documents were written to be read as text and hard-wrapped at a width: two things a Markdown renderer
    takes otherwise are put right here, outside fenced code. A line that wraps inside an inline code span is joined
    with the next (a renderer takes a continuation starting with `+`, `|` or `-` for a list or a table, and the
    newline would render as a space anyway); a bare `<word>` placeholder outside code is escaped, so that it is not
    taken for an HTML tag and dropped (the README's own `<picture>` and `<details>` stay HTML)."""
    out = []
    fence = False
    lines = text.split("\n")
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.lstrip().startswith("```"):
            fence = not fence
            out.append(line)
            i += 1
            continue
        if fence:
            out.append(line)
            i += 1
            continue
        while len(re.findall(r"(?<!`)`(?!`)", line)) % 2 == 1 and i + 1 < len(lines) and lines[i + 1].strip():
            line = line + " " + lines[i + 1].strip()
            i += 1
        parts = re.split(r"(`[^`]*`)", line)
        line = "".join(part if part.startswith("`") else re.sub(r"<(?!(?:picture|details|summary|br)>)([A-Za-z][A-Za-z0-9-]*)>", r"\\<\1>", part) for part in parts)
        out.append(line)
        i += 1
    return "\n".join(out)


def guess_lang(block):
    """The language of an untagged code fence, from its text: the documents' fences are shell sessions, Scala, the
    lock's YAML, JSON, TOML, JavaScript or Rust; whatever matches nothing stays plain."""
    head = block.lstrip()
    if re.match(r"(\$ |\./|teq |cargo |sbt |npm |npx |git |scala-cli |python3 |node |java |curl |cd |printf |echo |export |ssh |for |while |set )", head):
        return "bash"
    if re.search(r"^\s*(?:(?:case |final |sealed |abstract )*(?:class|object|trait|enum) [A-Z]\w*|def \w+\s*[(\[:=]|(?:lazy )?val \w+\s*[:=]|var \w+\s*[:=]|import scala\.|package [a-z]|given [\w\[]|extension \(|@main def|inline def|transparent inline)", block, re.M):
        return "scala"
    if re.match(r"\s*[\[{]", head) and re.search(r'"[^"]+"\s*:', block):
        return "json"
    if re.search(r"^\s*\[[A-Za-z.]+\]\s*$", block, re.M) or re.search(r'^[A-Za-z_-]+ = ["\[\d]', block, re.M):
        return "toml"
    if re.search(r"^\s*(import .+ from |export |const |let |function )", block, re.M):
        return "js"
    if re.search(r"^\s*(fn |let |impl |pub |struct |enum |use )", block, re.M):
        return "rust"
    if re.search(r"^(teq|format|binaries|projects|jars|repositories|inputs):", block, re.M):
        return "yaml"
    return ""


def tag_fences(text):
    """An untagged ``` fence gets the language guess_lang reads off its text, so that it is highlighted; a fence
    that closes a block is left alone."""
    out, lines, i, opened = [], text.split("\n"), 0, None
    while i < len(lines):
        line = lines[i]
        if line.lstrip().startswith("```"):
            if opened is None:
                opened = i
                if line.strip() == "```":
                    j = i + 1
                    while j < len(lines) and not lines[j].lstrip().startswith("```"):
                        j += 1
                    line = line + guess_lang("\n".join(lines[i + 1:j]))
            else:
                opened = None
        out.append(line)
        i += 1
    return "\n".join(out)


def caption_fences(text):
    """A fence whose first line is a comment naming a file (`// project/plugins.sbt`, `# teq.toml`) is wrapped in a
    figure captioned with that name, the comment dropped: the caption is the block's title bar on the page. A fence
    that names several files, one before each part, keeps its comments."""
    file_comment = r"^\s*(?://|#) ([\w./-]+\.\w+)\s*$"
    out, lines, i, opened = [], text.split("\n"), 0, False
    while i < len(lines):
        line = lines[i]
        name = re.match(file_comment, lines[i + 1]) if not opened and line.lstrip().startswith("```") and i + 1 < len(lines) else None
        if name:
            j = i + 2
            while j < len(lines) and not lines[j].lstrip().startswith("```"):
                j += 1
            body = lines[i + 2:j]
            if body and not body[0].strip():
                body = body[1:]
            if any(re.match(file_comment, b) for b in body):
                out.append(line)
                opened = True
                i += 1
                continue
            indent = line[:len(line) - len(line.lstrip())]
            out += [f'{indent}<figure class="code"><figcaption>{name.group(1)}</figcaption>', "", line, *body, lines[j] if j < len(lines) else indent + "```", "", f"{indent}</figure>"]
            i = j + 1
            continue
        if line.lstrip().startswith("```"):
            opened = not opened
        out.append(line)
        i += 1
    return "\n".join(out)


def points(text):
    """The intro's paragraphs after the lead, each an article: one that opens in bold (`**No JVM at compile
    time.** One native binary ...`) is titled by the bold, its period dropped, over the rest."""
    out = []
    for para in re.split(r"\n\s*\n", text.strip()):
        m = re.match(r"\*\*(.+?)\.?\*\*\s*(.*)", para, re.S)
        body = f"## {m.group(1)}\n\n{m.group(2)}" if m else para
        out.append(f"<article>\n\n{body}\n\n</article>")
    return "\n\n".join(out)


def labels(text):
    """A paragraph of a subsection that opens in bold (`**Open your project in Zed.** Install ...`) or is bold alone
    is a step: the bold becomes a heading over the rest."""
    out = []
    for para in text.split("\n\n"):
        m = re.match(r"\*\*(.+?)\*\*(?:\s+(.+))?\s*\Z", para, re.S)
        out.append(f"#### {m.group(1)}\n\n{m.group(2)}" if m and m.group(2) else f"#### {m.group(1)}" if m else para)
    return "\n\n".join(out)


# README.md: the intro (before the first `## `), its first paragraph apart from the rest, and a fragment per section.
# The compilation-speed section is split at its first paragraph and at its table, which the front page draws.
parts = re.split(r"^## ", readme, flags=re.M)
centred = r"\A(?:\s*<(?:p|h1) align=\"center\">.*?</(?:p|h1)>)+\s*"
intro = re.sub(centred, "", parts[0], flags=re.S).strip()
# The README's own line breaks (`<br>`) and disclosures (`<details>`) are the page's; the Markdown files for
# language models read without them, a disclosure's summary as a bold line over its text.
def unbroken(text):
    text = re.sub(r"<br\s*/?>", " ", text)
    text = re.sub(r"<details>\s*<summary>(.*?)</summary>", r"**\1**", text)
    return re.sub(r"\n*</details>", "", text)
intro_lead, _, more = intro.partition("\n\n")
write(os.path.join(staged, "readme", "intro.md"), front({"title": "intro"}) + link_docs(intro_lead))
write(os.path.join(staged, "readme", "intro-more.md"), front({"title": "intro (the rest)"}) + points(link_docs(more)))
sections = []
for part in parts[1:]:
    title, _, body = part.partition("\n")
    title = title.strip()
    slug = heading_slug(title)
    sections.append({"title": title, "slug": slug})
    items = re.split(r"^### ", body, flags=re.M)
    if len(items) > 1:
        # A section of `###` subsections, each a card or a column of the front page: its first paragraph is its
        # lead, the first paragraph after it that is one link alone is its link, set apart, and the rest is the
        # detail, a paragraph opening in bold a labelled step.
        write(os.path.join(staged, "readme", f"{slug}.md"), front({"title": title}) + link_docs(items[0]))
        entries = []
        for item in items[1:]:
            item_title, _, item_body = item.partition("\n")
            item_slug = heading_slug(item_title)
            lead, _, detail = item_body.strip("\n").partition("\n\n")
            paragraphs = detail.strip("\n").split("\n\n")
            link = next((m for m in (re.fullmatch(r"\s*\[([^\]]+)\]\(([^)]+)\)\s*", p) for p in paragraphs) if m), None)
            if link:
                path = link.group(2)
                href = f"/docs/{slugs[path]}/" if path in slugs else path
                detail = "\n\n".join(p for p in paragraphs if p is not link.string)
            entries.append({
                "slug": item_slug, "title": item_title.strip(), "lead": " ".join(lead.split()),
                "link": {"text": link.group(1), "href": href} if link else None,
            })
            write(os.path.join(staged, "readme", f"{slug}--{item_slug}.md"), front({"title": item_title.strip()}) + link_docs(labels(detail)))
        sections[-1]["items"] = entries
        continue
    if slug == "compilation-speed":
        # The first paragraph, and the rest of the text with the table taken out: the front page draws the table.
        table = re.search(r"(?:^\|[^\n]*\n?)+", body, re.M)
        lead, _, rest = body[:table.start()].strip("\n").partition("\n\n")
        write(os.path.join(staged, "readme", f"{slug}.md"), front({"title": title}) + link_docs(lead))
        notes = (rest.strip("\n") + "\n\n" + body[table.end():].strip("\n")).strip("\n")
        write(os.path.join(staged, "readme", f"{slug}-notes.md"), front({"title": title + " (notes)"}) + link_docs(notes))
    else:
        write(os.path.join(staged, "readme", f"{slug}.md"), front({"title": title}) + link_docs(body))
write(os.path.join(staged, "readme.json"), json.dumps(sections, indent=1))


# The speed table of the README's "Compilation speed" section: a row per input, the three times (its last three
# columns) as seconds, and what stands between the input and the times as the input's description.
def seconds(cell):
    m = re.match(r"([\d.]+)\s*(ms|s)", cell)
    v = float(m.group(1))
    return v / 1000 if m.group(2) == "ms" else v


speed = []
section = next(p for p in parts[1:] if p.startswith("Compilation speed"))
for line in section.splitlines():
    cells = [c.strip() for c in line.strip().strip("|").split("|")]
    if len(cells) >= 4 and re.match(r"[\d.]+\s*m?s$", cells[-3]):
        teq_time, _, faster = cells[-1].partition(",")
        speed.append({
            "input": cells[0], "kind": " ".join(cells[1:-3]),
            "cold": seconds(cells[-3]), "coldLabel": cells[-3],
            "warm": seconds(cells[-2]), "warmLabel": cells[-2],
            "teq": seconds(teq_time), "teqLabel": teq_time.strip(),
            "faster": faster.strip(),
        })
write(os.path.join(staged, "speed.json"), json.dumps(speed, indent=1))

# The documents: a page each, titled by its first heading, ordered as the table orders them.
titles = {}
for weight, (path, description) in enumerate(docs, 1):
    text = read(path)
    m = re.match(r"# (.+)\n", text)
    title = titles[path] = m.group(1).strip() if m else os.path.basename(path)
    body = text[m.end():] if m else text
    fields = {"title": title, "weight": weight, "source": path, "description": description}
    write(os.path.join(staged, "docs", f"{slugs[path]}.md"), front(fields) + link_docs(body))

# The README's name (the logo's alt text), tagline (the centred line) and headline (the centred h1, broken where
# the README breaks it), which the hero and every page's title and description start from, its lead (the intro's
# first paragraph), the version its plugin snippet names and the opening words of its status, which the hero's
# release line shows (.staged/site.json); and the same text for language models (llmstxt.org): the README and each
# document as Markdown files, /llms.txt listing them under the name and tagline, /llms-full.txt holding all of them.
# The README's centred logo, tagline and headline become a heading and two paragraphs.
name = re.search(r'<img alt="([^"]+)"', readme).group(1)
tagline = re.search(r'<p align="center">([^<]+)</p>', readme).group(1).strip()
h1 = re.search(r'<h1 align="center">(.*?)</h1>', readme, re.S)
headline = [" ".join(line.split()) for line in re.split(r"<br\s*/?>", h1.group(1))] if h1 else []
version = re.search(r"/releases/download/v([0-9]+\.[0-9]+\.[0-9]+)/", readme)
status = next((re.match(r"\s*#*\s*([^.,\n]+)", p[len("Status\n"):]) for p in parts[1:] if p.startswith("Status\n")), None)
write(os.path.join(staged, "site.json"), json.dumps({
    "name": name, "tagline": tagline, "headline": headline, "lead": " ".join(unbroken(intro_lead).split()).replace("`", ""),
    "version": version.group(1) if version else None, "status": status.group(1) if status else None,
}, indent=1))
opening = f"# {name}\n\n{tagline}\n\n" + (f"{' '.join(headline)}\n\n" if headline else "")
overview = opening + link_raw(unbroken(re.sub(centred, "", readme, flags=re.S)))
write(os.path.join(public, "index.md"), overview)
listing = [f"- [README]({site_url}/index.md): " + ", ".join(s["title"] for s in sections)]
full = [overview]
for path, description in docs:
    text = link_raw(read(path))
    write(os.path.join(public, "docs", f"{slugs[path]}.md"), text)
    summary = re.sub(rf"^{re.escape(titles[path])}: ", "", description, flags=re.I)
    listing.append(f"- [{titles[path]}]({site_url}/docs/{slugs[path]}.md): {summary}")
    full.append(text)
write(os.path.join(public, "llms.txt"), f"# {name}\n\n> {tagline}\n\n" + (f"{' '.join(headline)}\n\n" if headline else "") + f"{unbroken(intro)}\n\n## Docs\n\n" + "\n".join(listing) + "\n")
write(os.path.join(public, "llms-full.txt"), "\n\n".join(text.strip("\n") for text in full) + "\n")

# The logo, from the repository's assets.
logo = os.path.join(site, "public", "assets", "logo")
shutil.rmtree(logo, ignore_errors=True)
shutil.copytree(os.path.join(root, "assets", "logo"), logo)
shutil.copy(os.path.join(root, "assets", "logo", "teq-favicon-32.svg"), os.path.join(site, "public", "favicon.svg"))
# What an earlier run staged and this one did not (a section or document gone) is removed.
for folder, _, names in [*os.walk(staged), *os.walk(os.path.join(public, "docs"))]:
    for file in names:
        path = os.path.abspath(os.path.join(folder, file))
        if path not in written:
            os.remove(path)
print(f"staged README.md as {len(sections)} sections and {len(docs)} documents", file=sys.stderr)
