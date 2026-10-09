// The site's content is staged from the repository's Markdown by stage.scala (see site/README.md): the README's
// sections as `readme`, the documents the README's table names as `docs`.
import { defineCollection } from "astro:content";
import { glob } from "astro/loaders";
import { z } from "astro/zod";

const readme = defineCollection({
  loader: glob({ pattern: "**/*.md", base: "./.staged/readme" }),
  schema: z.object({ title: z.string() }),
});

const docs = defineCollection({
  loader: glob({ pattern: "**/*.md", base: "./.staged/docs" }),
  schema: z.object({
    title: z.string(),
    weight: z.number(),
    source: z.string(),
    description: z.string(),
  }),
});

export const collections = { readme, docs };
