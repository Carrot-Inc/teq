# teq.build

The site has no text of its own: `stage.scala` stages it from the repository's Markdown, so that the README and
`docs/` are the one source. The front page is `README.md` (`src/pages/index.astro` places its sections by their
headings' GitHub anchors; a section it does not place, a renamed one included, is appended, its subsections as
cards or its text alone): the centred tagline is the eyebrow, the centred `<h1>` the headline (its lines after the
README's `<br>` in the accent colour), the intro's first paragraph the lead under it and each paragraph after it a
point of the strip below, titled by its bold opening, with a release line that reads the compiler's version off the README's download links and the first sentence of
its Status section. The compilation-speed section is a card per row of its table, the input and the description
column beside it over the three times as outlined bars (teq's in green), the teq cell's ratio as a line under them, its first paragraph beside the
heading and the rest under the cards, a paragraph that is one link at the right; the ways to use teq (the section's `###` subsections) are a grid of cards, each its title, its
first paragraph and the rest, with a paragraph that is one link alone pinned to the card's foot, and quick links
under the heading that name each way by a short name the page keeps for its slug, a paragraph opening in bold a
step under its label (the two editor steps with their editors' marks from `src/icons/`), a list a two-column row
and a list of links a row of buttons; Status stands
under them, its subsection's title, first paragraph and link in the left column and the rest in the right (a list as
boxes); About the project and Open source side by side (each
its title as a label over its one subsection's title, first paragraph and the rest, a `<details>` block as GitHub
shows it), and the footer, the site's own line, names the license for the License section and links the
documentation, GitHub, the notices and the top of the page. A code fence whose first line is a comment naming a file
(`// build.sbt`) shows the name as the block's caption, and every block gets a copy button (the one script, in
`src/layouts/Base.astro`). The pages under `/docs/` are the documents the README's "Documentation" table names, in
its order, each titled by its first heading, with the page's headings beside it; a backticked `docs/X.md` anywhere
becomes a link to that page. To change what the site says, change the README or the document. To publish or
withdraw a document, edit the README's table. The same text is served to language models as Markdown
([llmstxt.org](https://llmstxt.org)): `/llms.txt` lists the README and the documents, `/llms-full.txt` holds all
of them, and each is a file of its own (`/index.md`, `/docs/<name>.md`).
The front page is titled by the README's name and headline and described by its tagline and lead, a document by
its first heading and its row of the table; the sitemap is `@astrojs/sitemap`'s, which `robots.txt` names, and the card a shared link shows (`/og.png`)
and the bookmark icon are the logo drawn to PNG at build time (`src/raster.ts`).

[Astro](https://astro.build) renders the staged content (`src/content.config.ts` names the two collections, the
pages are under `src/pages/`, the one stylesheet is `src/styles/style.css`):

```
cd site
npm install
npm run build      # stage.scala, then astro build into dist/
npm run dev        # stage.scala, then astro's dev server at http://localhost:4321
```

`stage.scala` runs under the repository's launcher (`../teq interp stage.scala`, the release `teq.lock` pins unless
`TEQ` names a binary) and reads and writes files alone, which the pinned release's interpreter has, so that a build
from a fresh clone runs it as it is.

`.staged/`, `dist/`, `node_modules/`, `.netlify/` and what `stage.scala` writes into `public/` (the logo and the Markdown
files) are git-ignored. The site deploys as the static directory `dist/`, from this directory:
`npx -y netlify-cli deploy --prod --no-build --dir dist` (the CLI would otherwise run the build itself). The
headers are `public/_headers`; their CSP admits scripts from the site only, so `astro.config.mjs` keeps the
copy-button script a file under `/_astro/` rather than inline. `netlify/edge-functions/password.js` asks every request for a password while the
project's `SITE_PASSWORD` variable is set. `netlify.toml` has the build for a repository connected to Netlify.
