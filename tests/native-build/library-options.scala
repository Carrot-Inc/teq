//> using file ../../tools/script
// Checks src/native/library-options.txt, the forms of a library's compile options the native build takes, against
// zig 0.16.0's option table, and writes tests/native-build/library-options.expected, the cases the suite
// (tests/native-build.scala) builds and src/native/library_options.rs's test answers. zig's table is its source's
// src/clang_options.zon at the release's tag, pinned below by URL and SHA-256; zig searches it in its order and takes
// the first entry an argument matches (src/main.zig, `ClangArgIterator.next`), as this script does. Each form is
// checked so:
//   - a token written whole: the entry zig takes it as is the one the form names, of the kind it names, and takes
//     no argument after it (a dependency output's option written `dropped+next` takes the one after it);
//   - a prefix and a value (`<prefix>*`): the entry is of a joined kind and spelt as the prefix, and no entry before
//     it in zig's order takes a token of the prefix and a value the form takes (the values it excepts, `!<value>` or
//     `!<start>*`, are those such entries take);
//   - an entry zig gives a meaning of its own (`.ze`) is one of `zigMeanings`, which shape the code alone.
// The cases: each token the table keeps or drops, with the values the suite names for a prefix (the fields `c:<v>`
// and `cpp:<v>`, or the token itself, compiled as C, or as C++ where the form says `cpp`); each value a form excepts;
// and refused, the review's options by name and a sample of zig's entries of each kind that no form takes. Run from
// the repository's root:
//
//   ./teq interp tests/native-build/library-options.scala -- [--zon <file>] [--check]
//
// The table is read from <file>, else from out/library-options/clang_options.zon, fetched there with curl when it is
// not; either is held to its digest first. --check writes nothing and fails (status 1) where the cases file is not
// what the tables give.
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, Path, Paths}

val zonUrl = "https://codeberg.org/ziglang/zig/raw/tag/0.16.0/src/clang_options.zon"
val zonSha256 = "2d589a35c427aa3f83d0d11855cfed6f92d6a25b80a856852298c7ac209656ee"
val table = Paths.get("src/native/library-options.txt")
val casesFile = Paths.get("tests/native-build/library-options.expected")

/// The meanings zig gives an option of its own that a kept form may have: those that shape the code it compiles, and
/// none that moves an input, an output, the language or the link.
val zigMeanings = Set("other", "optimize", "debug", "pic", "no_pic", "unwind_tables", "no_unwind_tables", "asynchronous_unwind_tables",
  "no_asynchronous_unwind_tables", "omit_frame_pointer", "no_omit_frame_pointer", "function_sections", "no_function_sections",
  "data_sections", "no_data_sections", "red_zone", "no_red_zone", "builtin", "no_builtin", "stack_check", "no_stack_check",
  "stack_protector", "no_stack_protector", "color_diagnostics", "no_color_diagnostics")

final case class Entry(at: Int, name: String, syntax: String, args: Int, ze: String, pd1: Boolean, pd2: Boolean, psl: Boolean):
  /// The spellings zig matches it by, in the order it tries them.
  def spellings: Vector[String] = Vector(Option.when(pd1)("-" + name), Option.when(pd2)("--" + name), Option.when(psl)("/" + name)).flatten
  def joined: Boolean = Set("joined", "comma_joined", "joined_or_separate", "joined_and_separate", "remaining_args_joined")(syntax)

/// zig's table: its entries in order, each `.{ ... }` with its fields.
def entries(zon: String): Vector[Entry] =
  val out = Vector.newBuilder[Entry]
  var depth = 0
  var start = -1
  var at = 0
  while at < zon.length do
    zon.charAt(at) match
      case '"' =>
        at += 1
        while zon.charAt(at) != '"' do at += (if zon.charAt(at) == '\\' then 2 else 1)
      case '/' if zon.startsWith("//", at) => at = zon.indexOf('\n', at)
      case '{' =>
        depth += 1
        if depth == 2 then start = at
      case '}' =>
        if depth == 2 then
          val body = zon.substring(start, at)
          def field(name: String): Option[String] = s"""\\.$name = ([^,}]+)""".r.findFirstMatchIn(body).map(_.group(1).trim)
          val name = """\.name = "((?:[^"\\]|\\.)*)"""".r.findFirstMatchIn(body).map(_.group(1)).getOrElse(Log.die(s"an entry without a name: $body"))
          val multi = """\.multi_arg = (\d+)""".r.findFirstMatchIn(body).map(_.group(1).toInt)
          val syntax = if multi.isDefined then "multi_arg" else field("syntax").map(_.stripPrefix(".")).getOrElse("flag")
          val args = syntax match
            case "separate" | "joined_and_separate" => 1
            case "multi_arg" => multi.get
            case _ => 0
          out += Entry(0, name, syntax, args, field("ze").map(_.stripPrefix(".")).getOrElse("other"),
            field("pd1").forall(_ == "true"), field("pd2").contains("true"), field("psl").contains("true"))
        depth -= 1
      case _ =>
    at += 1
  out.result().zipWithIndex.map((e, i) => e.copy(at = i))

/// What zig takes an argument as (`ClangArgIterator.next`): the entry and how many arguments after it it takes; `None`
/// for an input (an argument that does not start with `-`, or `-` alone).
def zigTakes(zig: Vector[Entry], arg: String): Option[(Entry, Int)] =
  if arg == "-" || !arg.startsWith("-") then None
  else
    zig.iterator.flatMap { e =>
      val eql = e.spellings.contains(arg)
      val starts = e.spellings.find(arg.startsWith)
      e.syntax match
        case "flag" if eql => Some(e -> 0)
        case "joined" | "comma_joined" if starts.isDefined => Some(e -> 0)
        case "joined_or_separate" if starts.isDefined => Some(e -> (if starts.get.length == arg.length then 1 else 0))
        case "joined_and_separate" if starts.isDefined => Some(e -> 1)
        case "separate" if eql => Some(e -> 1)
        case "multi_arg" if eql => Some(e -> e.args)
        case "remaining_args_joined" if starts.isDefined => Some(e -> Int.MaxValue)
        case _ => None
    }.nextOption()

final case class Form(spelling: String, prefix: Boolean, role: String, entry: String, syntax: String, except: Vector[(String, Boolean)],
    samples: Vector[(String, String)], cpp: Boolean):
  def takes(token: String): Boolean =
    if !prefix then token == spelling
    else token.startsWith(spelling) && {
      val value = token.substring(spelling.length)
      value.nonEmpty && !except.exists((e, start) => if start then value.startsWith(e) else value == e)
    }

def star(word: String): (String, Boolean) = if word.endsWith("*") then (word.dropRight(1), true) else (word, false)

/// The table's forms; what library_options.rs reads of a row (its first two fields and the exceptions) and what this
/// script checks (the entry, its kind, the samples).
def forms(text: String): Vector[Form] =
  text.linesIterator.filter(l => l.nonEmpty && !l.startsWith("#")).map { line =>
    val f = line.trim.split("\\s+").toVector
    if f.length < 4 then Log.die(s"$table: a row without an entry and its kind: $line")
    val (spelling, prefix) = star(f(0))
    val rest = f.drop(4)
    Form(spelling, prefix, f(1), f(2), f(3), rest.filter(_.startsWith("!")).map(w => star(w.drop(1))),
      rest.filter(w => w.startsWith("c:") || w.startsWith("cpp:")).map(w => (w.takeWhile(_ != ':'), w.dropWhile(_ != ':').drop(1))),
      rest.contains("cpp"))
  }.toVector

/// library_options.rs's choice: the form written as the token, else the one of the longest spelling that takes it.
def formOf(all: Vector[Form], token: String): Option[Form] =
  all.find(f => !f.prefix && f.spelling == token).orElse(all.filter(f => f.prefix && f.takes(token)).maxByOption(_.spelling.length))

/// What the build answers of a library's options: `passed` (zig given them all), `dropped` (none), else `refused`
/// where the first it does not take is the first.
def answer(all: Vector[Form], tokens: Vector[String]): String =
  var kept = Vector.empty[String]
  var at = 0
  var refused: Option[String] = None
  while at < tokens.length && refused.isEmpty do
    formOf(all, tokens(at)).map(_.role) match
      case Some("kept") => kept :+= tokens(at)
      case Some("dropped") =>
      case Some("dropped+next") if at + 1 < tokens.length => at += 1
      case _ => refused = Some(tokens(at))
    at += 1
  refused match
    case Some(t) if t == tokens.head => "refused"
    case Some(t) => s"refused $t"
    case None if kept == tokens => "passed"
    case None if kept.isEmpty => "dropped"
    case None => s"kept ${kept.mkString(",")}"

@main def libraryOptions(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/native-build/library-options.scala [--zon <file>] [--check]")
  val zonGiven = args.option("zon")
  val check = args.flag("check")
  args.exactly(0)
  val zonPath = zonGiven.map(Paths.get(_)).getOrElse {
    val p = Paths.get("out/library-options/clang_options.zon")
    if !Files.isRegularFile(p) then
      Files.createDirectories(p.getParent)
      Sh("curl", "-sSfL", "--max-time", "120", "-o", p.toString, zonUrl).run()
    p
  }
  val zonBytes = Files.readAllBytes(zonPath)
  val digest = java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(zonBytes))
  if digest != zonSha256 then Log.die(s"$zonPath has sha256 $digest, not zig 0.16.0's $zonSha256")
  val zig = entries(new String(zonBytes, UTF_8))
  val all = forms(Files.readString(table))
  var faults = Vector.empty[String]
  def fault(why: String): Unit = faults :+= why

  // The forms against zig's table.
  for form <- all do
    val named = zig.filter(e => e.name == form.entry && e.syntax == form.syntax)
    if named.isEmpty then fault(s"${form.spelling}: zig has no entry ${form.entry} of kind ${form.syntax}")
    if !Set("kept", "dropped", "dropped+next")(form.role) then fault(s"${form.spelling}: no role ${form.role}")
    if !form.prefix then
      val wanted = if form.role == "dropped+next" then 1 else 0
      zigTakes(zig, form.spelling) match
        case Some((e, n)) if e.name == form.entry && e.syntax == form.syntax && n == wanted =>
          if form.role == "kept" && !zigMeanings(e.ze) then fault(s"${form.spelling}: zig takes it as ${e.ze}")
        case Some((e, n)) => fault(s"${form.spelling}: zig takes it as ${e.name} (${e.syntax}, ${n} after it), not ${form.entry} (${form.syntax}) with $wanted")
        case None => fault(s"${form.spelling}: zig takes it as an input")
    else
      named.find(_.spellings.contains(form.spelling)) match
        case None => fault(s"${form.spelling}*: no entry ${form.entry} of kind ${form.syntax} is spelt ${form.spelling}")
        case Some(own) =>
          if !own.joined || own.args > 0 || form.role == "dropped+next" then fault(s"${form.spelling}*: ${own.name} is not of a joined kind that takes nothing after it")
          if form.role == "kept" && !zigMeanings(own.ze) then fault(s"${form.spelling}*: zig takes it as ${own.ze}")
          for e <- zig.take(own.at); s <- e.spellings do
            val excepted = (value: String) => form.except.exists((x, start) => if start then value.startsWith(x) else value == x)
            if e.joined && form.spelling.startsWith(s) then fault(s"${form.spelling}*: zig takes every such token as ${e.name}, before ${own.name}")
            else if s.startsWith(form.spelling) && s.length > form.spelling.length then
              val value = s.substring(form.spelling.length)
              val covered = if e.joined then form.except.exists((x, start) => start && value.startsWith(x)) else excepted(value)
              if !covered then fault(s"${form.spelling}*: zig takes ${s}${if e.joined then "..." else ""} as ${e.name}, before ${own.name}: the form does not except it")
          if form.prefix && form.role == "kept" && form.samples.isEmpty then fault(s"${form.spelling}*: no sample value")

  // The cases.
  val values = Vector("hash#target", "dollar$target", "a blank", "colon:target")
  val cases = Vector.newBuilder[(String, String, Vector[String])]
  def add(language: String, tokens: Vector[String]): Unit = cases += ((answer(all, tokens), language, tokens))
  for form <- all do
    val language = if form.cpp then "cpp" else "c"
    (form.role, form.prefix) match
      case ("kept", false) => add(language, Vector(form.spelling))
      case ("kept", true) => form.samples.foreach((l, v) => add(l, Vector(form.spelling + v)))
      case ("dropped", false) => add("c", Vector(form.spelling))
      case ("dropped", true) => values.foreach(v => add("c", Vector(form.spelling + v)))
      case _ => values.foreach(v => add("c", Vector(form.spelling, v)))
    for (x, start) <- form.except do add("c", Vector(form.spelling + x + (if start then "x" else "")))
  // The review's options by name, and the options of the cut's first form refused or looked into before.
  val named = Vector(
    "--include-directory=/abs/shA", "-isystem-after,/abs/shB", "-I,-MD,-DFOO=1", "-iprefix,-MD,-DFOO=1", "-Xarch_,-MD,-DFOO=1", "-I,-MD",
    "-header-include-file,-DFOO=1", "@more.rsp", "--config,my.cfg", "--config=my.cfg", "--config-system-dir=/etc/clang",
    "--config-user-dir=/clang", "-Xclang,-MD", "-Xclang=-MD", "-Xpreprocessor,-MD", "-Xarch_host,-MD", "-Xarch_device,-MD", "-Xarch_x86_64,-MD",
    "-Wp", "-sys-header-deps", "-module-file-deps", "-header-include-format=json", "-header-include-filtering=only-direct-system", "--show-includes",
    "-include,extra.h", "-imacros,extra.h", "-working-directory,/abs", "-include-pch,a.pch", "-fmodules", "-fmodule-map-file=a.modulemap",
    "-fprebuilt-module-path=/abs", "--embed-dir=/abs", "-fsanitize-ignorelist=a.txt", "-fprofile-list=a.txt", "-ffile-prefix-map=/=/x/",
    "-fdebug-prefix-map=/=/x/", "-fmacro-prefix-map=/=/x/", "-fdebug-compilation-dir=/x", "-no-canonical-prefixes", "-save-temps", "-iquote,/abs",
    "-isystem,/abs", "-idirafter,/abs", "-x,c", "-flto", "-flto=thin", "-emit-llvm", "-fsyntax-only", "-E", "-S", "-###", "-o,a.o", "-c",
    "foo.c", "-", "", "-fprofile-sample-use=a.prof", "-fplugin=a.so", "-Wl,-z,now", "-L/abs", "-lfoo", "-target,x86_64-linux-gnu", "--sysroot=/abs")
  for options <- named do add("c", options.split(",", -1).toVector)
  // Of each kind of zig's entries, four that no form takes, spelt as zig matches them with a value where they take one.
  for syntax <- zig.map(_.syntax).distinct.sorted do
    val untaken = zig.filter(e => e.syntax == syntax && (e.pd1 || e.pd2)).map { e =>
      val s = e.spellings.head
      e -> (syntax match
        case "flag" => Vector(s)
        case "separate" | "multi_arg" => s +: Vector.fill(e.args)("x")
        case "joined_and_separate" => Vector(s + "x", "y")
        case _ => Vector(s + "x"))
    }.filter((_, tokens) => answer(all, tokens).startsWith("refused"))
    for k <- 0 until 4 if untaken.nonEmpty do add("c", untaken(k * untaken.length / 4)._2)
  val made = cases.result().distinct
  for (expected, _, tokens) <- made if expected.contains(' ') do fault(s"${tokens.mkString(",")}: the build would answer `$expected`")
  for kind <- Vector("passed", "dropped", "refused") if !made.exists(_._1 == kind) do fault(s"no case $kind")
  faults.foreach(f => println(s"FAIL $f"))
  if faults.nonEmpty then Script.exit(1)

  val text = (Vector(
    "# Written by tests/native-build/library-options.scala from src/native/library-options.txt and zig 0.16.0's",
    s"# src/clang_options.zon (sha256 $zonSha256): `<answer> <language> <options>`, the options as a",
    "# library's descriptor writes them, the answer the build's (passed: zig is given each as it is; dropped: none;",
    "# refused: the first, by name), the language the suite compiles them in.") ++
    made.map((answer, language, tokens) => (s"$answer $language " + tokens.mkString(",")).trim)).mkString("", "\n", "\n")
  if check then
    val same = Files.isRegularFile(casesFile) && Files.readString(casesFile) == text
    println(s"library-options: $casesFile ${if same then "holds" else "does not hold"} the cases the tables give (${made.length})")
    if !same then Script.exit(1)
  else
    Files.writeString(casesFile, text)
    println(s"library-options: ${all.length} forms checked against zig's ${zig.length} entries; ${made.length} cases written to $casesFile")
}
