//> using file ../../tools/script
// The reduction survey's baseline: what the tree is made of and where it repeats itself, measured the same way
// at every commit, so that a candidate's size and a spike's diff are read against one reference. Run from the
// repository's root (tokei 15.0.0 on the PATH, node and npm for jscpd, curl for the registry):
//
//   ./teq interp bench/survey/survey.scala -- [--out <dir>] [--only <part>]... [--crates <file>] [--jscpd <dir>]
//   ./teq interp bench/survey/survey.scala -- --check
//
// --check runs the script's own checks and measures nothing: the function scan over fixed sources (an array in a
// signature, a declaration, generics with `->`, a const generic, a function's name in a string or a comment) and the
// rule below on reusing a baseline, over every pair of states; it prints a line per failure and the count, status 1
// on a failure.
//
// The tree measured is the working directory's: a revision's own figures come from a checkout of it, the script
// and its configuration named by path from another (`--crates`, `--jscpd`), so that the survey's own files are
// not among what it counts. Each part writes its raw output under <dir> (default out/survey) and its line on stdout;
// `baseline.json` there holds the revision, whether the tree was clean, the tools' versions and every part's
// totals. Clean means no tracked file changed and no untracked file (one git does not ignore) under the
// measured trees (src, std, tests, bench), since the measurements read those as they read tracked ones. A run into a
// directory that holds a baseline adds its parts to it only when the baseline was recorded clean at the same
// revision, and this tree is clean too; otherwise it refuses,
// naming both states, and measures nothing, since the parts it did not run would stand under this run's revision: a
// baseline of another revision, or of a dirty tree (two dirty states of one commit cannot be told apart, so a dirty
// measurement is never merged into, only replaced), or a dirty tree now. The caller removes the
// directory (or names another) to measure afresh. The parts:
//
// tokei      tokei's JSON per tree (src, std, tests, bench: tokei-<tree>.json), per language the code lines, the
//            comment lines and tokei's physical lines (code, comments and blanks), the languages embedded in the
//            language's files counted with its comments at every depth (Rust's documentation comments are Markdown,
//            which can hold TOML or Scala in its turn), and apart from them the raw lines of its files as read
//            (`wc -l`), which tokei's count need not equal
// clones     jscpd (the version the lock in --jscpd's directory pins, default bench/survey/jscpd's, installed
//            under <dir>/jscpd) over src/ with the Rust tokenizer, default mode, at 50 and 100 tokens
//            (clones-<n>/jscpd-report.json); the families of each: pairs joined where they share a site (file,
//            first and last line), ranked by the largest pair's tokens times the sites past the first
//            (families-<n>.json, top-families-<n>.md), a triage order and not a count of removable lines
// functions  every `fn` of src/ with a body, its first and last line (functions.json) and the thirty longest
//            (longest-30.tsv): comments, strings and character literals masked, braces matched, the signature read
//            to its body's `{` or a declaration's `;` outside its parentheses, brackets and angle brackets (an
//            array type's `;`, `[u8; 64]`, is the signature's); a lexical scan, nested functions counted apart, not a
//            parser
// crates     each crate the file names (default bench/survey/crates.txt) from crates.io's API: its newest stable
//            version, licence, archive size, release date and declared dependencies that are not dev ones
//            (optional and build edges included), crates/<name>.json beside crates/summary.json; the registry's
//            declarations, not the graph a build resolves
//
// The counts are the tools' and nothing here decides a candidate: bench/survey/README.md has the reading.
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, Path, Paths}

@main def survey(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/survey/survey.scala [--out <dir>] [--only tokei|clones|functions|crates]... [--crates <file>] [--jscpd <dir>] | --check")
  if args.flag("check") then
    args.exactly(0)
    Script.exit(Checks.run())
  val out = Paths.get(args.option("out").getOrElse("out/survey"))
  val cratesFile = Paths.get(args.option("crates").getOrElse("bench/survey/crates.txt"))
  val jscpdLock = Paths.get(args.option("jscpd").getOrElse("bench/survey/jscpd"))
  val all = Vector("tokei", "clones", "functions", "crates")
  var only = Vector.empty[String]
  var more = args.option("only")
  while more.isDefined do
    only :+= more.get
    more = args.option("only")
  only.filterNot(all.contains).foreach(p => args.fail(s"no part $p"))
  args.exactly(0)
  val parts = if only.isEmpty then all else only
  Files.createDirectories(out)
  val baselineFile = out.resolve("baseline.json")
  var baseline = if Files.exists(baselineFile) then Json.parse(Files.readString(baselineFile)) else Json.obj()
  def tool(cmd: String*): String =
    val r = Sh(cmd*).check(false).timeout(60).run()
    if r.ok then r.out.linesIterator.nextOption().getOrElse("").trim else "absent"
  val head = tool("git", "rev-parse", "HEAD")
  // Clean: no tracked file changed, and no file the measurements read that git does not track (an untracked file
  // under the measured trees, which tokei, jscpd and the scan read as they read a tracked one).
  val clean = Sh("git", "status", "--porcelain", "--untracked-files=no").check(false).timeout(60).run().out.trim.isEmpty &&
    Sh(("git" +: "status" +: "--porcelain" +: "--untracked-files=all" +: "--" +: Tokei.trees)*).check(false).timeout(60).run().out.trim.isEmpty
  for recorded <- baseline.get("revision"); why <- Reuse.refusal(recorded.str, baseline.get("clean").forall(_.bool), head, clean) do
    Log.die(s"survey: $baselineFile $why; remove $out, or name another --out, to measure afresh")
  baseline = baseline
    .updated("revision", Json.str(head))
    .updated("clean", Json.bool(clean))
    .updated("date", Json.str(Log.utc("yyyy-MM-dd'T'HH:mm:ss'Z'")))
    .updated("tools", Json.obj(
      "tokei" -> Json.str(tool("tokei", "--version")),
      "node" -> Json.str(tool("node", "--version")),
      "rustc" -> Json.str(tool("rustc", "--version")),
      "curl" -> Json.str(tool("curl", "--version"))
    ))
  for part <- parts do
    val (line, summary) = part match
      case "tokei" => Tokei.run(out)
      case "clones" => Clones.run(out, jscpdLock)
      case "functions" => Functions.run(out)
      case "crates" => Crates.run(out, cratesFile)
    println(s"$part: $line")
    baseline = baseline.updated(part, summary)
  Files.writeString(baselineFile, Json.write(baseline, indent = 2) + "\n")
  println(s"baseline: ${baselineFile} at ${Reuse.state(head, clean)}")
}

def readJson(p: Path): Json = Json.parse(new String(Files.readAllBytes(p), UTF_8))

// Whether a run may add its parts to a baseline: only to one recorded at this revision with the tracked files as
// committed, from a tree that is so too; else why not.
object Reuse:
  def state(revision: String, clean: Boolean): String =
    revision + (if clean then "" else " with a tracked file changed or an untracked file under the measured trees")

  def refusal(recorded: String, recordedClean: Boolean, head: String, clean: Boolean): Option[String] =
    if !recordedClean then
      Some(s"was recorded at ${state(recorded, false)}, and a dirty tree's measurement is never merged into, since two " +
        "dirty states of one commit cannot be told apart: it is only replaced")
    else if recorded != head || !clean then
      Some(s"was recorded at ${state(recorded, true)} and this tree is at ${state(head, clean)}: its parts and this run's " +
        "would read as one tree's")
    else None

object Tokei:
  val trees = Vector("src", "std", "tests", "bench")

  def run(out: Path): (String, Json) =
    val rows = trees.map { tree =>
      val file = out.resolve(s"tokei-$tree.json")
      Sh("tokei", tree, "--output", "json").stdout(file).timeout(300).run()
      val languages = readJson(file).members.filter(_._1 != "Total").map { (language, v) =>
        // A language's embedded languages (Rust's documentation comments are Markdown) are its children, and
        // each child's lines hold the blobs embedded in it in their turn.
        val embedded = v("children").members.flatMap(_._2.elements).map(r => lines(r("stats"))).sum
        val code = v("code").long
        val comments = v("comments").long + embedded
        val physical = code + comments + v("blanks").long
        val raw = v("reports").elements.map(r => rawLines(Paths.get(r("name").str))).sum
        language -> Json.obj("files" -> Json.num(v("reports").elements.length), "code" -> Json.num(code), "comments" -> Json.num(comments),
          "physical" -> Json.num(physical), "raw" -> Json.num(raw))
      }
      tree -> Json.Obj(languages.sortBy(-_._2("code").long))
    }
    val src = rows.find(_._1 == "src").get._2("Rust")
    (s"src/ Rust ${src("code").long} code lines, ${src("physical").long} physical (tokei's), ${src("raw").long} in its files as read", Json.Obj(rows))

  // A stats record's lines, its blobs' at every depth among them.
  def lines(stats: Json): Long =
    stats("code").long + stats("comments").long + stats("blanks").long + stats.get("blobs").map(_.members.map((_, b) => lines(b)).sum).getOrElse(0L)

  // A file's lines as `wc -l` counts them: its newlines.
  def rawLines(p: Path): Long =
    val bytes = Files.readAllBytes(p)
    var n = 0L
    var i = 0
    while i < bytes.length do
      if bytes(i) == '\n'.toByte then n += 1
      i += 1
    n

object Clones:
  val thresholds = Vector(50, 100)

  def run(out: Path, lock: Path): (String, Json) =
    val tools = out.resolve("jscpd")
    Files.createDirectories(tools)
    for f <- Vector("package.json", "package-lock.json") do
      Files.copy(lock.resolve(f), tools.resolve(f), java.nio.file.StandardCopyOption.REPLACE_EXISTING)
    val bin = tools.resolve("node_modules/.bin/jscpd")
    if !Files.exists(bin) then
      Sh("npm", "ci", "--no-audit", "--no-fund", "--silent").in(tools).timeout(300).run()
    val version = Sh(bin.toString, "--version").timeout(60).run().out.trim
    val rows = thresholds.map { n =>
      val dir = out.resolve(s"clones-$n")
      Sh.remove(dir)
      Sh(bin.toString, "src", "--format", "rust", "--min-tokens", n.toString, "--reporters", "json", "--output", dir.toString, "--silent")
        .timeout(300).run()
      val report = readJson(dir.resolve("jscpd-report.json"))
      val total = report("statistics")("total")
      val families = group(report("duplicates").elements)
      Files.writeString(out.resolve(s"families-$n.json"), Json.write(Json.Arr(families.map(_.json)), indent = 2) + "\n")
      Files.writeString(out.resolve(s"top-families-$n.md"), table(families.take(10)))
      n.toString -> Json.obj(
        "pairs" -> total("clones"),
        "duplicatedLines" -> total("duplicatedLines"),
        "duplicatedTokens" -> total("duplicatedTokens"),
        "percentage" -> total("percentage"),
        "sources" -> total("sources"),
        "lines" -> total("lines"),
        "families" -> Json.num(families.length)
      )
    }
    val line = rows.map((n, s) => s"$n tokens ${s("pairs").long} pairs, ${s("duplicatedLines").long} lines").mkString("; ")
    (s"$version: $line", Json.Obj(Vector("jscpd" -> Json.str(version)) ++ rows))

  final case class Site(file: String, start: Int, end: Int)
  final case class Family(sites: Vector[Site], tokens: Long, lines: Long, pairs: Int):
    def redundant: Long = tokens * (sites.length - 1)
    def json: Json = Json.obj(
      "tokens" -> Json.num(tokens),
      "redundant_tokens" -> Json.num(redundant),
      "sites" -> Json.Arr(sites.map(s => Json.arr(Json.str(s.file), Json.num(s.start), Json.num(s.end)))),
      "lines" -> Json.num(lines),
      "pairs" -> Json.num(pairs)
    )

  def site(f: Json): Site = Site(f("name").str, f("start").int, f("end").int)

  // The pairs joined by a shared site (union-find over the sites), each family's sites sorted.
  def group(duplicates: Vector[Json]): Vector[Family] =
    val parent = scala.collection.mutable.HashMap.empty[Site, Site]
    def find(s: Site): Site =
      var r = s
      while parent.getOrElse(r, r) != r do r = parent(r)
      parent(s) = r
      r
    val pairs = duplicates.map(d => (site(d("firstFile")), site(d("secondFile")), d("tokens").long, d("lines").long))
    for (a, b, _, _) <- pairs do
      val (ra, rb) = (find(a), find(b))
      if ra != rb then parent(ra) = rb
    val byRoot = pairs.groupBy(p => find(p._1))
    byRoot.values.toVector.map { ps =>
      val sites = ps.flatMap(p => Vector(p._1, p._2)).distinct.sortBy(s => (s.file, s.start, s.end))
      Family(sites, ps.map(_._3).max, ps.map(_._4).max, ps.length)
    }.sortBy(f => (-f.redundant, -f.tokens, f.sites.head.file, f.sites.head.start))

  def table(families: Vector[Family]): String =
    val rows = families.zipWithIndex.map { (f, i) =>
      val sites = f.sites.map(s => s"`src/${s.file}:${s.start}–${s.end}`").mkString("; ")
      s"| ${i + 1} | ${f.tokens} × ${f.sites.length - 1} = ${f.redundant} | ${f.lines} | $sites |"
    }
    ("| Family | Tokens × additional sites | Largest span | All sites |" +: "|---|---:|---:|---|" +: rows).mkString("", "\n", "\n")

object Functions:
  final case class Row(file: String, name: String, start: Int, end: Int):
    def lines: Int = end - start + 1

  // The source with its comments, strings and character literals turned to spaces, its newlines kept.
  def mask(s: String): Array[Char] =
    val m = s.toCharArray
    val n = m.length
    def blank(from: Int, to: Int): Unit =
      var k = from
      while k < to && k < n do
        if m(k) != '\n' then m(k) = ' '
        k += 1
    def ident(c: Char): Boolean = c.isLetterOrDigit || c == '_'
    var i = 0
    while i < n do
      val c = s.charAt(i)
      if c == '/' && i + 1 < n && s.charAt(i + 1) == '/' then
        var j = i
        while j < n && s.charAt(j) != '\n' do j += 1
        blank(i, j)
        i = j
      else if c == '/' && i + 1 < n && s.charAt(i + 1) == '*' then
        var depth = 1
        var j = i + 2
        while j < n && depth > 0 do
          if s.charAt(j) == '/' && j + 1 < n && s.charAt(j + 1) == '*' then { depth += 1; j += 2 }
          else if s.charAt(j) == '*' && j + 1 < n && s.charAt(j + 1) == '/' then { depth -= 1; j += 2 }
          else j += 1
        blank(i, j)
        i = j
      else if (c == 'r' || c == 'b') && (i == 0 || !ident(s.charAt(i - 1))) && rawStart(s, i) >= 0 then
        val (open, hashes) = (rawStart(s, i), rawHashes(s, i))
        val close = "\"" + "#" * hashes
        val j = s.indexOf(close, open + 1)
        val end = if j < 0 then n else j + close.length
        blank(i, end)
        i = end
      else if c == '"' then
        var j = i + 1
        while j < n && s.charAt(j) != '"' do j += (if s.charAt(j) == '\\' then 2 else 1)
        blank(i, j + 1)
        i = j + 1
      else if c == '\'' && !(i > 0 && ident(s.charAt(i - 1)) && s.charAt(i - 1) != 'b') then
        val j = charEnd(s, i)
        if j > 0 then
          blank(i, j)
          i = j
        else i += 1
      else i += 1
    m

  // Where a raw string's quote is (`r"`, `r#"`, `br##"`), or -1.
  def rawStart(s: String, i: Int): Int =
    var j = i
    if s.charAt(j) == 'b' then j += 1
    if j >= s.length || s.charAt(j) != 'r' then return -1
    j += 1
    while j < s.length && s.charAt(j) == '#' do j += 1
    if j < s.length && s.charAt(j) == '"' then j else -1

  def rawHashes(s: String, i: Int): Int =
    var j = rawStart(s, i) - 1
    var k = 0
    while s.charAt(j) == '#' do { k += 1; j -= 1 }
    k

  // The end of a character literal at `i` (`'x'`, `'\n'`, `'\u{1F600}'`, `'\x7f'`), or -1 for a lifetime.
  def charEnd(s: String, i: Int): Int =
    val n = s.length
    if i + 2 >= n then return -1
    val c = s.charAt(i + 1)
    if c == '\\' then
      var j = i + 2
      if s.charAt(j) == 'u' && j + 1 < n && s.charAt(j + 1) == '{' then
        j = s.indexOf('}', j)
        if j < 0 then return -1
        j += 1
      else if s.charAt(j) == 'x' then j += 3
      else j += 1
      if j < n && s.charAt(j) == '\'' then j + 1 else -1
    else if c == '\'' || c == '\n' then -1
    else
      // A supplementary character is two UTF-16 units.
      val w = if Character.isHighSurrogate(c) then 2 else 1
      if i + 1 + w < n && s.charAt(i + 1 + w) == '\'' then i + 2 + w else -1

  val fnPattern = java.util.regex.Pattern.compile("\\bfn\\s+(\\w+)\\s*(?:<|\\()")

  def scan(file: String, s: String): Vector[Row] =
    val m = mask(s)
    val masked = new String(m)
    val pairs = new Array[Int](m.length)
    val stack = new Array[Int](m.length.max(1))
    var depth = 0
    var i = 0
    while i < m.length do
      pairs(i) = -1
      if m(i) == '{' then { stack(depth) = i; depth += 1 }
      else if m(i) == '}' && depth > 0 then { depth -= 1; pairs(stack(depth)) = i }
      i += 1
    // The line of each offset, by the offsets of the newlines.
    val newlines = scala.collection.mutable.ArrayBuffer.empty[Int]
    i = 0
    while i < m.length do
      if m(i) == '\n' then newlines += i
      i += 1
    def lineOf(at: Int): Int =
      var (lo, hi) = (0, newlines.length)
      while lo < hi do
        val mid = (lo + hi) / 2
        if newlines(mid) < at then lo = mid + 1 else hi = mid
      lo + 1
    val rows = Vector.newBuilder[Row]
    val f = fnPattern.matcher(masked)
    while f.find() do
      val b = bodyStart(masked, f.end() - 1)
      if b >= 0 && pairs(b) >= 0 then
        rows += Row(file, f.group(1), lineOf(f.start()), lineOf(pairs(b)))
    rows.result()

  // Where the body of a signature read from `from` (its `<` or `(`) opens: the offset of the `{`, or -1 for a
  // declaration (a `;`) or a text that ends first. Only a `{` or `;` outside the signature's parentheses, brackets and
  // angle brackets counts (`[u8; 64]`, `[u8; { N }]` are the signature's); the `>` of `->` and `=>` closes nothing.
  def bodyStart(m: String, from: Int): Int =
    var (paren, bracket, angle) = (0, 0, 0)
    var i = from
    var found = -2
    while found == -2 && i < m.length do
      val c = m.charAt(i)
      val open = paren == 0 && bracket == 0 && angle == 0
      if c == '(' then paren += 1
      else if c == ')' then paren -= 1
      else if c == '[' then bracket += 1
      else if c == ']' then bracket -= 1
      else if c == '<' then angle += 1
      else if c == '>' then
        if angle > 0 && !(i > 0 && (m.charAt(i - 1) == '-' || m.charAt(i - 1) == '=')) then angle -= 1
      else if c == '{' && open then found = i
      else if c == ';' && open then found = -1
      i += 1
    if found == -2 then -1 else found

  def sources(dir: Path): Vector[Path] =
    val s = Files.walk(dir)
    try s.toArray.toVector.map(_.asInstanceOf[Path]).filter(p => p.toString.endsWith(".rs") && Files.isRegularFile(p)).sortBy(_.toString)
    finally s.close()

  def run(out: Path): (String, Json) =
    val files = sources(Paths.get("src"))
    val rows = files.flatMap(p => scan(p.toString, new String(Files.readAllBytes(p), UTF_8)))
      .sortBy(r => (-r.lines, r.file, r.start))
    val json = Json.Arr(rows.map(r => Json.obj("file" -> Json.str(r.file), "name" -> Json.str(r.name), "start" -> Json.num(r.start), "end" -> Json.num(r.end), "lines" -> Json.num(r.lines))))
    Files.writeString(out.resolve("functions.json"), Json.write(json, indent = 2) + "\n")
    val top = rows.take(30)
    Files.writeString(out.resolve("longest-30.tsv"), ("lines\tfunction\tsite" +: top.map(r => s"${r.lines}\t${r.name}\t${r.file}:${r.start}-${r.end}")).mkString("", "\n", "\n"))
    val total = top.map(_.lines).sum
    (s"${rows.length} functions in ${files.length} files; the thirty longest ${total} lines, the longest ${top.head.name} (${top.head.lines})",
      Json.obj("functions" -> Json.num(rows.length), "files" -> Json.num(files.length), "longest30Lines" -> Json.num(total)))

object Crates:
  val agent = "teq-reduction-survey (bench/survey/survey.scala)"

  def get(url: String, file: Path): Json =
    Sh("curl", "-sS", "--fail", "--max-time", "30", "-A", agent, "-o", file.toString, url).timeout(60).run()
    readJson(file)

  def run(out: Path, list: Path): (String, Json) =
    val names = Files.readAllLines(list).toArray.toVector.map(_.toString.trim).filter(l => l.nonEmpty && !l.startsWith("#"))
    val dir = out.resolve("crates")
    Files.createDirectories(dir)
    val rows = names.map { name =>
      val crate = get(s"https://crates.io/api/v1/crates/$name", dir.resolve(s"$name.json"))
      val version = crate("crate")("max_stable_version").str
      val v = crate("versions").elements.find(_("num").str == version).get
      val deps = get(s"https://crates.io/api/v1/crates/$name/$version/dependencies", dir.resolve(s"$name-dependencies.json"))
      val declared = deps("dependencies").elements.filter(_("kind").str != "dev")
      Json.obj(
        "name" -> Json.str(name),
        "version" -> Json.str(version),
        "license" -> v("license"),
        "bytes" -> v("crate_size"),
        "released" -> v("created_at"),
        "repository" -> crate("crate")("repository"),
        "dependencies" -> Json.num(declared.length),
        "declared" -> Json.Arr(declared.map(d => Json.str(s"${d("crate_id").str} ${d("req").str} ${d("kind").str}${if d("optional").bool then " optional" else ""}")))
      )
    }
    Files.writeString(dir.resolve("summary.json"), Json.write(Json.Arr(rows), indent = 2) + "\n")
    (s"${rows.length} crates from the registry", Json.Arr(rows.map(r => Json.obj("name" -> r("name"), "version" -> r("version"), "license" -> r("license"), "bytes" -> r("bytes"), "dependencies" -> r("dependencies")))))

// The script's own checks (`--check`): the function scan over fixed sources, the reuse rule over every pair of states.
object Checks:
  def run(): Int =
    var failed = 0
    var passed = 0
    def expect(what: String, got: Any, want: Any): Unit =
      if got == want then passed += 1
      else
        failed += 1
        println(s"FAIL $what: $got, not $want")
    def scanned(src: String): Vector[(String, Int, Int)] = Functions.scan("check.rs", src).map(r => (r.name, r.start, r.end))
    expect("a one-line function", scanned("fn one() { }\n"), Vector(("one", 1, 1)))
    expect("an array in a parameter", scanned("fn compress(&mut self, block: &[u8; 64]) {\n    let x = 1;\n}\n"), Vector(("compress", 1, 3)))
    expect("an array in the result", scanned("fn table() -> [u32; 256] {\n    [0; 256]\n}\n"), Vector(("table", 1, 3)))
    expect("a declaration, then a body", scanned("trait T {\n    fn decl(x: [u8; 4]);\n    fn after(&self) {}\n}\n"), Vector(("after", 3, 3)))
    expect("generics with arrows", scanned("fn gen<T: Fn(u8) -> [u8; 2]>(t: T) -> Box<dyn Fn() -> u8>\nwhere\n    T: Copy,\n{\n    t;\n}\n"), Vector(("gen", 1, 6)))
    expect("a const generic", scanned("fn konst<const N: usize>(x: [u8; N]) -> [u8; { N }] {\n    x\n}\n"), Vector(("konst", 1, 3)))
    expect("a nested function", scanned("fn outer() {\n    fn inner() {}\n}\n").sortBy(_._1), Vector(("inner", 2, 2), ("outer", 1, 3)))
    expect("names in a string, a raw string and a comment", scanned("// fn c() {\nfn real() { let s = \"fn s() {\"; let r = r#\"fn r() {\"#; }\n"), Vector(("real", 2, 2)))
    val states = for r <- Vector("a", "b"); c <- Vector(true, false) yield (r, c)
    for (rr, rc) <- states; (hr, hc) <- states do
      val reused = Reuse.refusal(rr, rc, hr, hc).isEmpty
      expect(s"reuse of a baseline at ${Reuse.state(rr, rc)} from ${Reuse.state(hr, hc)}", reused, rr == hr && rc && hc)
    println(s"survey checks: $passed passed, $failed failed")
    if failed == 0 then 0 else 1
