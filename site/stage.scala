// site/stage.scala: stages the site's content from the repository's Markdown, so that the site has no text of its
// own.
//
// README.md is split at its `## ` headings into fragments the front page composes (src/pages/index.astro names the
// order): the intro's first paragraph is the hero's lead and each paragraph after it a point of the strip under the
// hero, titled by its bold opening, and the compilation-speed section's first paragraph stands beside its heading,
// the rest of its text under the cards drawn from its table (.staged/speed.json). The documents the README's
// "Documentation" table names become the pages under /docs/, in the table's order, each titled by its first heading.
// A backticked `docs/X.md` in any of them becomes a link to that page where the page exists, and a code fence whose
// first line is a comment naming a file gets that name as its caption. The name, the tagline, the headline (the
// README's centred h1, its lines kept apart), the lead, the plugin's version and the opening words of the status
// (.staged/site.json) are what the hero and the page titles read. Everything lands under site/.staged/
// (git-ignored), and the logo is copied from assets/logo/ into site/public/assets/ (git-ignored too), beside the
// Markdown files for language models (/llms.txt, /llms-full.txt, /index.md, /docs/*.md). `npm run build` runs this
// first, from site/ (`../teq interp stage.scala`); from the repository's root it is `./teq interp site/stage.scala`.
//
// It reads and writes files alone, through what the pinned release's interpreter has, so that a build from a fresh
// clone (Netlify's) runs it under the lock's teq; the patterns are written as Python's were and the JSON as Python's
// `json.dumps` wrote it, so that the staged files are the bytes stage.py staged.
import java.nio.file.{Files, Path, Paths}
import java.util.regex.{Matcher, Pattern}

object Stage:
  // ---- the Python idioms the staging is written in ----

  def compile(p: String, flags: Int = 0): Pattern = Pattern.compile(p, flags)
  val S = Pattern.DOTALL
  val M = Pattern.MULTILINE

  // `re.search`: the first match anywhere.
  def search(p: Pattern, s: String): Option[Matcher] =
    val m = p.matcher(s)
    if m.find() then Some(m) else None
  // `re.match`: a match at the start.
  def matchStart(p: Pattern, s: String): Option[Matcher] =
    val m = p.matcher(s)
    if m.lookingAt() then Some(m) else None
  // `re.fullmatch`.
  def fullMatch(p: Pattern, s: String): Option[Matcher] =
    val m = p.matcher(s)
    if m.matches() then Some(m) else None
  // `re.sub` with a function: each match replaced by what it gives, the rest kept.
  def sub(p: Pattern, s: String)(f: Matcher => String): String =
    val m = p.matcher(s)
    val sb = new java.lang.StringBuilder()
    var at = 0
    while m.find() do
      sb.append(s, at, m.start())
      sb.append(f(m))
      at = m.end()
    sb.append(s, at, s.length)
    sb.toString
  // `re.findall` of a pattern without groups.
  def findAll(p: Pattern, s: String): Vector[String] =
    val m = p.matcher(s)
    val out = Vector.newBuilder[String]
    while m.find() do out += m.group()
    out.result()
  // `re.split`: the pieces between the matches, with the groups' texts between them; an empty piece
  // kept at either end.
  def split(p: Pattern, s: String): Vector[String] =
    val m = p.matcher(s)
    val out = Vector.newBuilder[String]
    var at = 0
    while m.find() do
      out += s.substring(at, m.start())
      var g = 1
      while g <= m.groupCount() do
        out += m.group(g)
        g += 1
      at = m.end()
    out += s.substring(at)
    out.result()
  // `str.partition`.
  def partition(s: String, sep: String): (String, String, String) =
    val at = s.indexOf(sep)
    if at < 0 then (s, "", "") else (s.substring(0, at), sep, s.substring(at + sep.length))
  // `str.isspace` of a character, `str.strip`, `str.split()` with no separator.
  def space(c: Char): Boolean =
    c == ' ' || (c >= '\t' && c <= '\r') || (c >= '\u001c' && c <= '\u001f') || c == '\u0085' || c == ' ' || c == ' ' ||
      (c >= ' ' && c <= ' ') || c == ' ' || c == ' ' || c == ' ' || c == ' ' || c == '　'
  def strip(s: String): String =
    var a = 0
    var b = s.length
    while a < b && space(s.charAt(a)) do a += 1
    while b > a && space(s.charAt(b - 1)) do b -= 1
    s.substring(a, b)
  def stripChars(s: String, chars: String): String =
    var a = 0
    var b = s.length
    while a < b && chars.indexOf(s.charAt(a)) >= 0 do a += 1
    while b > a && chars.indexOf(s.charAt(b - 1)) >= 0 do b -= 1
    s.substring(a, b)
  def lstrip(s: String): String =
    var a = 0
    while a < s.length && space(s.charAt(a)) do a += 1
    s.substring(a)
  def words(s: String): Vector[String] =
    val out = Vector.newBuilder[String]
    var i = 0
    while i < s.length do
      while i < s.length && space(s.charAt(i)) do i += 1
      val start = i
      while i < s.length && !space(s.charAt(i)) do i += 1
      if i > start then out += s.substring(start, i)
    out.result()
  // `str.splitlines()`, enough for the README's table: at `\n`, `\r\n` and `\r`, no empty piece at the end.
  def splitlines(s: String): Vector[String] =
    val parts = s.split("\r\n|\r|\n", -1).toVector
    if parts.nonEmpty && parts.last.isEmpty then parts.init else parts

  // ---- JSON as Python's `json.dumps` writes it ----

  def quote(s: String): String =
    val sb = new java.lang.StringBuilder("\"")
    for c <- s do
      c match
        case '"' => sb.append("\\\"")
        case '\\' => sb.append("\\\\")
        case '\n' => sb.append("\\n")
        case '\r' => sb.append("\\r")
        case '\t' => sb.append("\\t")
        case '\b' => sb.append("\\b")
        case '\f' => sb.append("\\f")
        case _ if c < 0x20 || c > 0x7e =>
          val hex = Integer.toHexString(c)
          sb.append("\\u").append("0000".substring(hex.length)).append(hex)
        case _ => sb.append(c)
    sb.append('"').toString

  // A double as Python's `repr` writes it (the shortest digits; fixed from 1e-4 to below 1e16).
  def pythonFloat(d: Double): String =
    if d == 0.0 then (if 1.0 / d < 0 then "-0.0" else "0.0")
    else
      val shortest = java.lang.Double.toString(Math.abs(d))
      val e = shortest.indexOf('E')
      val (mantissa, exp) = if e < 0 then (shortest, 0) else (shortest.substring(0, e), shortest.substring(e + 1).toInt)
      val point = mantissa.indexOf('.')
      val whole = mantissa.substring(0, point)
      val all = whole + mantissa.substring(point + 1)
      val first = all.indexWhere(_ != '0')
      val digits = all.substring(first).reverse.dropWhile(_ == '0').reverse match
        case "" => "0"
        case ds => ds
      val e10 = exp + whole.length - first - 1
      val sign = if d < 0 then "-" else ""
      if e10 < -4 || e10 >= 16 then
        val m = if digits.length == 1 then digits else digits.substring(0, 1) + "." + digits.substring(1)
        val es = Math.abs(e10).toString
        sign + m + "e" + (if e10 < 0 then "-" else "+") + (if es.length < 2 then "0" + es else es)
      else if e10 < 0 then sign + "0." + ("0" * (-e10 - 1)) + digits
      else if digits.length <= e10 + 1 then sign + digits + ("0" * (e10 + 1 - digits.length)) + ".0"
      else sign + digits.substring(0, e10 + 1) + "." + digits.substring(e10 + 1)

  // A value: a String, an Int, a Double, null, a Seq of values or a Seq of (String, value) pairs.
  final case class Obj(fields: Seq[(String, Any)])
  def dumps(v: Any, indent: Int = -1, depth: Int = 0): String =
    def nl(d: Int): String = if indent >= 0 then "\n" + (" " * (indent * d)) else ""
    val sep = if indent >= 0 then "," else ", "
    v match
      case null => "null"
      case s: String => quote(s)
      case i: Int => i.toString
      case d: Double => pythonFloat(d)
      case b: Boolean => if b then "true" else "false"
      case Obj(fields) =>
        if fields.isEmpty then "{}"
        else "{" + fields.map((k, x) => nl(depth + 1) + quote(k) + ": " + dumps(x, indent, depth + 1)).mkString(sep) + nl(depth) + "}"
      case xs: Seq[?] =>
        if xs.isEmpty then "[]"
        else "[" + xs.map(x => nl(depth + 1) + dumps(x, indent, depth + 1)).mkString(sep) + nl(depth) + "]"
      case other => throw new IllegalArgumentException("no JSON for " + other)

  // ---- the staging ----

  def main(args: Array[String]): Unit =
    val here = Paths.get("").toAbsolutePath
    val site = if Files.exists(here.resolve("astro.config.mjs")) then here else here.resolve("site")
    val root = site.getParent
    val staged = site.resolve(".staged")
    val public = site.resolve("public")
    val siteUrl = search(compile("site: \"([^\"]+)\""), Files.readString(site.resolve("astro.config.mjs"))).get.group(1).reverse.dropWhile(_ == '/').reverse

    def read(path: String): String = Files.readString(root.resolve(path))

    val written = scala.collection.mutable.Set.empty[Path]

    // Writes a staged file only when its text changed, so that a dev server watching .staged/ sees an edit as an
    // edit; the directory is never emptied and rebuilt, which its content store does not follow.
    def write(path: Path, text: String): Unit =
      written += path.toAbsolutePath.normalize
      Files.createDirectories(path.getParent)
      val same = Files.exists(path) && !Files.isDirectory(path) && Files.readString(path) == text
      if !same then Files.writeString(path, text)

    def front(fields: Seq[(String, Any)]): String =
      "---\n" + fields.map((k, v) => s"$k: ${dumps(v)}\n").mkString + "---\n\n"

    val readme = read("README.md")

    // The documents to publish: every `docs/...` path of the README's "Documentation" table, in the table's order,
    // with the row's description. A directory lists its .md files.
    val docs = scala.collection.mutable.ArrayBuffer.empty[(String, String)]
    val table = search(compile("^## Documentation\n(.*?)(?=^## |\\z)", S | M), readme).get.group(1)
    for line <- splitlines(table) do
      val cells = stripChars(strip(line), "|").split("\\|", -1).toVector.map(strip)
      if cells.length == 2 && (cells(0).startsWith("`") || cells(0).startsWith("[")) then
        for path <- findAll(compile("docs/[A-Za-z0-9_./-]+"), cells(0)).distinct do
          if path.endsWith("/") then
            val listing = Files.list(root.resolve(path))
            val names = try listing.toArray.map(_.asInstanceOf[Path].getFileName.toString).sorted finally listing.close()
            for name <- names if name.endsWith(".md") do docs += ((path + name, cells(1)))
          else docs += ((path, cells(1)))

    def slugOf(path: String): String =
      stripChars(compile("[^a-z0-9]+").matcher(path.substring("docs/".length, path.length - ".md".length).toLowerCase).replaceAll("-"), "-")

    // A heading's anchor as GitHub makes it: lowercased, punctuation dropped, spaces as hyphens (Python's `\w` and
    // `\s`, letters, digits, `_` and white space, kept).
    def headingSlug(title: String): String =
      val sb = new java.lang.StringBuilder()
      for c <- strip(title).toLowerCase do
        if Character.isLetterOrDigit(c) || c == '_' || space(c) || c == '-' then sb.append(if c == ' ' then '-' else c)
      sb.toString

    val slugs: Map[String, String] = docs.map((path, _) => path -> slugOf(path)).toMap

    def linkPaths(text: String, url: String => String): String =
      // stage.py's `(?<!\]\()`: a path already a link's target is left alone (tested by hand, a lookbehind
      // being slow in the pinned release's patterns).
      val coded = sub(compile("`(docs/[A-Za-z0-9_./-]+\\.md)`"), text) { m =>
        val path = m.group(1)
        val target = m.start() >= 2 && text.startsWith("](", m.start() - 2)
        if slugs.contains(path) && !target then s"[`$path`](${url(slugs(path))})" else m.group()
      }
      sub(compile("\\]\\((docs/[A-Za-z0-9_./-]+\\.md)\\)"), coded) { m =>
        val path = m.group(1)
        if slugs.contains(path) then s"](${url(slugs(path))})" else m.group()
      }

    // The documents were written to be read as text and hard-wrapped at a width: two things a Markdown renderer takes
    // otherwise are put right here, outside fenced code. A line that wraps inside an inline code span is joined with
    // the next (a renderer takes a continuation starting with `+`, `|` or `-` for a list or a table, and the newline
    // would render as a space anyway); a bare `<word>` placeholder outside code is escaped, so that it is not taken
    // for an HTML tag and dropped (the README's own `<picture>` and `<details>` stay HTML).
    def markdownSafe(text: String): String =
      val out = Vector.newBuilder[String]
      var fence = false
      val lines = text.split("\n", -1)
      val lone = compile("(?<!`)`(?!`)")
      val placeholder = compile("<(?!(?:picture|details|summary|br)>)([A-Za-z][A-Za-z0-9-]*)>")
      var i = 0
      while i < lines.length do
        var line = lines(i)
        if lstrip(line).startsWith("```") then
          fence = !fence
          out += line
          i += 1
        else if fence then
          out += line
          i += 1
        else
          while findAll(lone, line).length % 2 == 1 && i + 1 < lines.length && strip(lines(i + 1)).nonEmpty do
            line = line + " " + strip(lines(i + 1))
            i += 1
          val parts = split(compile("(`[^`]*`)"), line)
          line = parts.map(part => if part.startsWith("`") then part else sub(placeholder, part)(m => "\\<" + m.group(1) + ">")).mkString
          out += line
          i += 1
      out.result().mkString("\n")

    // The language of an untagged code fence, from its text: the documents' fences are shell sessions, Scala, the
    // lock's YAML, JSON, TOML, JavaScript or Rust; whatever matches nothing stays plain.
    def guessLang(block: String): String =
      val head = lstrip(block)
      if matchStart(compile("(\\$ |\\./|teq |cargo |sbt |npm |npx |git |scala-cli |python3 |node |java |curl |cd |printf |echo |export |ssh |for |while |set )"), head).isDefined then "bash"
      else if search(compile("^\\s*(?:(?:case |final |sealed |abstract )*(?:class|object|trait|enum) [A-Z]\\w*|def \\w+\\s*[(\\[:=]|(?:lazy )?val \\w+\\s*[:=]|var \\w+\\s*[:=]|import scala\\.|package [a-z]|given [\\w\\[]|extension \\(|@main def|inline def|transparent inline)", M), block).isDefined then "scala"
      else if matchStart(compile("\\s*[\\[{]"), head).isDefined && search(compile("\"[^\"]+\"\\s*:"), block).isDefined then "json"
      else if search(compile("^\\s*\\[[A-Za-z.]+\\]\\s*$", M), block).isDefined || search(compile("^[A-Za-z_-]+ = [\"\\[\\d]", M), block).isDefined then "toml"
      else if search(compile("^\\s*(import .+ from |export |const |let |function )", M), block).isDefined then "js"
      else if search(compile("^\\s*(fn |let |impl |pub |struct |enum |use )", M), block).isDefined then "rust"
      else if search(compile("^(teq|format|binaries|projects|jars|repositories|inputs):", M), block).isDefined then "yaml"
      else ""

    // An untagged ``` fence gets the language guessLang reads off its text, so that it is highlighted; a fence that
    // closes a block is left alone.
    def tagFences(text: String): String =
      val lines = text.split("\n", -1)
      val out = Vector.newBuilder[String]
      var opened = false
      var i = 0
      while i < lines.length do
        var line = lines(i)
        if lstrip(line).startsWith("```") then
          if !opened then
            opened = true
            if strip(line) == "```" then
              var j = i + 1
              while j < lines.length && !lstrip(lines(j)).startsWith("```") do j += 1
              line = line + guessLang(lines.slice(i + 1, j).mkString("\n"))
          else opened = false
        out += line
        i += 1
      out.result().mkString("\n")

    // A fence whose first line is a comment naming a file (`// project/plugins.sbt`, `# teq.toml`) is wrapped in a
    // figure captioned with that name, the comment dropped: the caption is the block's title bar on the page. A fence
    // that names several files, one before each part, keeps its comments.
    def captionFences(text: String): String =
      val fileComment = compile("^\\s*(?://|#) ([\\w./-]+\\.\\w+)\\s*$")
      val lines = text.split("\n", -1)
      val out = Vector.newBuilder[String]
      var opened = false
      var i = 0
      while i < lines.length do
        val line = lines(i)
        val name = if !opened && lstrip(line).startsWith("```") && i + 1 < lines.length then matchStart(fileComment, lines(i + 1)) else None
        name match
          case Some(n) =>
            var j = i + 2
            while j < lines.length && !lstrip(lines(j)).startsWith("```") do j += 1
            var body = lines.slice(i + 2, j).toVector
            if body.nonEmpty && strip(body.head).isEmpty then body = body.tail
            if body.exists(b => matchStart(fileComment, b).isDefined) then
              out += line
              opened = true
              i += 1
            else
              val indent = line.substring(0, line.length - lstrip(line).length)
              out += s"""$indent<figure class="code"><figcaption>${n.group(1)}</figcaption>"""
              out += ""
              out += line
              body.foreach(out += _)
              out += (if j < lines.length then lines(j) else indent + "```")
              out += ""
              out += s"$indent</figure>"
              i = j + 1
          case None =>
            if lstrip(line).startsWith("```") then opened = !opened
            out += line
            i += 1
      out.result().mkString("\n")

    // `docs/X.md` and [..](docs/X.md) become links to the rendered page; an unpublished path stays as it is.
    def linkDocs(text: String): String = linkPaths(captionFences(tagFences(markdownSafe(text))), slug => s"/docs/$slug/")
    // The same links for the Markdown files served to language models, to the files themselves.
    def linkRaw(text: String): String = linkPaths(text, slug => s"$siteUrl/docs/$slug.md")

    // The intro's paragraphs after the lead, each an article: one that opens in bold (`**No JVM at compile time.**
    // One native binary ...`) is titled by the bold, its period dropped, over the rest.
    def points(text: String): String =
      split(compile("\n\\s*\n"), strip(text)).map { para =>
        val body = matchStart(compile("\\*\\*(.+?)\\.?\\*\\*\\s*(.*)", S), para) match
          case Some(m) => s"## ${m.group(1)}\n\n${m.group(2)}"
          case None => para
        s"<article>\n\n$body\n\n</article>"
      }.mkString("\n\n")

    // A paragraph of a subsection that opens in bold (`**Open your project in Zed.** Install ...`) or is bold alone is
    // a step: the bold becomes a heading over the rest.
    def labels(text: String): String =
      text.split("\n\n", -1).map { para =>
        fullMatch(compile("\\*\\*(.+?)\\*\\*(?:\\s+(.+))?\\s*", S), para) match
          case Some(m) if m.group(2) != null => s"#### ${m.group(1)}\n\n${m.group(2)}"
          case Some(m) => s"#### ${m.group(1)}"
          case None => para
      }.mkString("\n\n")

    // README.md: the intro (before the first `## `), its first paragraph apart from the rest, and a fragment per
    // section. The compilation-speed section is split at its first paragraph and at its table, which the front page
    // draws.
    val parts = split(compile("^## ", M), readme)
    val centred = compile("\\A(?:\\s*<(?:p|h1) align=\"center\">.*?</(?:p|h1)>)+\\s*", S)
    val intro = strip(centred.matcher(parts(0)).replaceFirst(""))
    // The README's own line breaks (`<br>`) and disclosures (`<details>`) are the page's; the Markdown files for
    // language models read without them, a disclosure's summary as a bold line over its text.
    def unbroken(text: String): String =
      val t1 = compile("<br\\s*/?>").matcher(text).replaceAll(" ")
      val t2 = sub(compile("<details>\\s*<summary>(.*?)</summary>"), t1)(m => s"**${m.group(1)}**")
      compile("\n*</details>").matcher(t2).replaceAll("")
    val (introLead, _, more) = partition(intro, "\n\n")
    write(staged.resolve("readme").resolve("intro.md"), front(Seq("title" -> "intro")) + linkDocs(introLead))
    write(staged.resolve("readme").resolve("intro-more.md"), front(Seq("title" -> "intro (the rest)")) + points(linkDocs(more)))
    val sections = scala.collection.mutable.ArrayBuffer.empty[scala.collection.mutable.ArrayBuffer[(String, Any)]]
    for part <- parts.drop(1) do
      val (rawTitle, _, body) = partition(part, "\n")
      val title = strip(rawTitle)
      val slug = headingSlug(title)
      val section = scala.collection.mutable.ArrayBuffer[(String, Any)]("title" -> title, "slug" -> slug)
      sections += section
      val items = split(compile("^### ", M), body)
      if items.length > 1 then
        // A section of `###` subsections, each a card or a column of the front page: its first paragraph is its
        // lead, the first paragraph after it that is one link alone is its link, set apart, and the rest is the
        // detail, a paragraph opening in bold a labelled step.
        write(staged.resolve("readme").resolve(s"$slug.md"), front(Seq("title" -> title)) + linkDocs(items(0)))
        val entries = scala.collection.mutable.ArrayBuffer.empty[Obj]
        for item <- items.drop(1) do
          val (itemTitle, _, itemBody) = partition(item, "\n")
          val itemSlug = headingSlug(itemTitle)
          val (lead, _, detail0) = partition(stripChars(itemBody, "\n"), "\n\n")
          val paragraphs = stripChars(detail0, "\n").split("\n\n", -1).toVector
          val linked = paragraphs.indices.iterator.flatMap(k => fullMatch(compile("\\s*\\[([^\\]]+)\\]\\(([^)]+)\\)\\s*"), paragraphs(k)).map(m => (k, m))).nextOption()
          var detail = detail0
          val link = linked.map { (k, m) =>
            val path = m.group(2)
            val href = if slugs.contains(path) then s"/docs/${slugs(path)}/" else path
            detail = paragraphs.indices.filter(_ != k).map(paragraphs).mkString("\n\n")
            Obj(Seq("text" -> m.group(1), "href" -> href))
          }
          entries += Obj(Seq("slug" -> itemSlug, "title" -> strip(itemTitle), "lead" -> words(lead).mkString(" "), "link" -> link.orNull))
          write(staged.resolve("readme").resolve(s"$slug--$itemSlug.md"), front(Seq("title" -> strip(itemTitle))) + linkDocs(labels(detail)))
        section += ("items" -> entries.toVector)
      else if slug == "compilation-speed" then
        // The first paragraph, and the rest of the text with the table taken out: the front page draws the table.
        val t = search(compile("(?:^\\|[^\\n]*\\n?)+", M), body).get
        val (lead, _, rest) = partition(stripChars(body.substring(0, t.start()), "\n"), "\n\n")
        write(staged.resolve("readme").resolve(s"$slug.md"), front(Seq("title" -> title)) + linkDocs(lead))
        val notes = stripChars(stripChars(rest, "\n") + "\n\n" + stripChars(body.substring(t.end()), "\n"), "\n")
        write(staged.resolve("readme").resolve(s"$slug-notes.md"), front(Seq("title" -> (title + " (notes)"))) + linkDocs(notes))
      else write(staged.resolve("readme").resolve(s"$slug.md"), front(Seq("title" -> title)) + linkDocs(body))
    write(staged.resolve("readme.json"), dumps(sections.toVector.map(s => Obj(s.toVector)), indent = 1))

    // The speed table of the README's "Compilation speed" section: a row per input, the three times (its last three
    // columns) as seconds, and what stands between the input and the times as the input's description.
    def seconds(cell: String): Double =
      val m = matchStart(compile("([\\d.]+)\\s*(ms|s)"), cell).get
      val v = m.group(1).toDouble
      if m.group(2) == "ms" then v / 1000 else v
    val speed = scala.collection.mutable.ArrayBuffer.empty[Obj]
    val speedSection = parts.drop(1).find(_.startsWith("Compilation speed")).get
    for line <- splitlines(speedSection) do
      val cells = stripChars(strip(line), "|").split("\\|", -1).toVector.map(strip)
      if cells.length >= 4 && fullMatch(compile("[\\d.]+\\s*m?s"), cells(cells.length - 3)).isDefined then
        val (teqTime, _, faster) = partition(cells.last, ",")
        speed += Obj(Seq(
          "input" -> cells(0), "kind" -> cells.slice(1, cells.length - 3).mkString(" "),
          "cold" -> seconds(cells(cells.length - 3)), "coldLabel" -> cells(cells.length - 3),
          "warm" -> seconds(cells(cells.length - 2)), "warmLabel" -> cells(cells.length - 2),
          "teq" -> seconds(teqTime), "teqLabel" -> strip(teqTime),
          "faster" -> strip(faster)
        ))
    write(staged.resolve("speed.json"), dumps(speed.toVector, indent = 1))

    // The documents: a page each, titled by its first heading, ordered as the table orders them.
    val titles = scala.collection.mutable.Map.empty[String, String]
    for ((path, description), weight) <- docs.zipWithIndex.map((d, i) => (d, i + 1)) do
      val text = read(path)
      val m = matchStart(compile("# (.+)\n"), text)
      val title = m.map(x => strip(x.group(1))).getOrElse(Paths.get(path).getFileName.toString)
      titles(path) = title
      val body = m.fold(text)(x => text.substring(x.end()))
      val fields = Seq("title" -> title, "weight" -> weight, "source" -> path, "description" -> description)
      write(staged.resolve("docs").resolve(s"${slugs(path)}.md"), front(fields) + linkDocs(body))

    // The README's name (the logo's alt text), tagline (the centred line) and headline (the centred h1, broken where
    // the README breaks it), which the hero and every page's title and description start from, its lead (the intro's
    // first paragraph), the version its plugin snippet names and the opening words of its status, which the hero's
    // release line shows (.staged/site.json); and the same text for language models (llmstxt.org): the README and
    // each document as Markdown files, /llms.txt listing them under the name and tagline, /llms-full.txt holding all
    // of them. The README's centred logo, tagline and headline become a heading and two paragraphs.
    val name = search(compile("<img alt=\"([^\"]+)\""), readme).get.group(1)
    val tagline = strip(search(compile("<p align=\"center\">([^<]+)</p>"), readme).get.group(1))
    val h1 = search(compile("<h1 align=\"center\">(.*?)</h1>", S), readme)
    val headline = h1.map(m => split(compile("<br\\s*/?>"), m.group(1)).map(l => words(l).mkString(" "))).getOrElse(Vector.empty)
    val version = search(compile("/releases/download/v([0-9]+\\.[0-9]+\\.[0-9]+)/"), readme).map(_.group(1))
    val status = parts.drop(1).find(_.startsWith("Status\n")).flatMap(p => matchStart(compile("\\s*#*\\s*([^.,\\n]+)"), p.substring("Status\n".length))).map(_.group(1))
    write(staged.resolve("site.json"), dumps(Obj(Seq(
      "name" -> name, "tagline" -> tagline, "headline" -> headline, "lead" -> words(unbroken(introLead)).mkString(" ").replace("`", ""),
      "version" -> version.orNull, "status" -> status.orNull
    )), indent = 1))
    val opening = s"# $name\n\n$tagline\n\n" + (if headline.nonEmpty then s"${headline.mkString(" ")}\n\n" else "")
    val overview = opening + linkRaw(unbroken(centred.matcher(readme).replaceFirst("")))
    write(public.resolve("index.md"), overview)
    val listing = scala.collection.mutable.ArrayBuffer(s"- [README]($siteUrl/index.md): " + sections.map(_.head._2.toString).mkString(", "))
    val full = scala.collection.mutable.ArrayBuffer(overview)
    for (path, description) <- docs do
      val text = linkRaw(read(path))
      write(public.resolve("docs").resolve(s"${slugs(path)}.md"), text)
      val summary = Pattern.compile("^" + Pattern.quote(titles(path)) + ": ", Pattern.CASE_INSENSITIVE).matcher(description).replaceFirst("")
      listing += s"- [${titles(path)}]($siteUrl/docs/${slugs(path)}.md): $summary"
      full += text
    write(public.resolve("llms.txt"), s"# $name\n\n> $tagline\n\n" + (if headline.nonEmpty then s"${headline.mkString(" ")}\n\n" else "") + s"${unbroken(intro)}\n\n## Docs\n\n" + listing.mkString("\n") + "\n")
    write(public.resolve("llms-full.txt"), full.map(t => stripChars(t, "\n")).mkString("\n\n") + "\n")

    // The logo, from the repository's assets.
    def remove(p: Path): Unit =
      if Files.exists(p) then
        val all =
          val s = Files.walk(p)
          try s.toArray.toVector.map(_.asInstanceOf[Path]) finally s.close()
        all.reverse.foreach(Files.deleteIfExists)
    def copyTree(from: Path, to: Path): Unit =
      val all =
        val s = Files.walk(from)
        try s.toArray.toVector.map(_.asInstanceOf[Path]) finally s.close()
      for f <- all do
        val target = to.resolve(from.relativize(f).toString)
        if Files.isDirectory(f) then Files.createDirectories(target)
        else Files.write(target, Files.readAllBytes(f))
    val logo = site.resolve("public").resolve("assets").resolve("logo")
    remove(logo)
    copyTree(root.resolve("assets").resolve("logo"), logo)
    Files.write(site.resolve("public").resolve("favicon.svg"), Files.readAllBytes(root.resolve("assets").resolve("logo").resolve("teq-favicon-32.svg")))
    // What an earlier run staged and this one did not (a section or document gone) is removed.
    for folder <- List(staged, public.resolve("docs")) if Files.isDirectory(folder) do
      val all =
        val s = Files.walk(folder)
        try s.toArray.toVector.map(_.asInstanceOf[Path]) finally s.close()
      for f <- all if !Files.isDirectory(f) && !written.contains(f.toAbsolutePath.normalize) do Files.deleteIfExists(f)
    System.err.println(s"staged README.md as ${sections.length} sections and ${docs.length} documents")
