//> using scala 3.8.4
//> using dep org.scalameta:scalameta_2.13:4.17.4
//> using dep org.scala-lang:scala3-compiler_3:3.8.4
//> using file RegionStream.scala
// The corpus of the node contract (tests/fix/scalameta, read by `nodes::tests::scalameta_corpus`): the programs
// scalameta 4.17.4's own parser tests assert a tree of, for the Scala 3 dialect, every suite under
// tests/shared/src/test/scala/scala/meta/tests/parsers/dotty of a checkout at v4.17.4. A program is the literal a call
// parses and asserts what it gives (`runTestAssert`, `parseAndCheckTree`, `checkStat`, `matchSubStructure`, the suites'
// `assertTpe`, `assertPat` and `assertPatTyp`, and `stat`, `templStat`, `blockStat`, `topStat`, `term`, `pat`,
// `patternTyp`, `tpe`, `source`, `parseCase` over one; `runTestError`'s left out), a `val` before the call naming it
// resolved; with its suite, its test's name, what it parses as and the dialect it parses under (the call's own
// argument, else the nearest `implicit val dialect` before it in an enclosing block, else the suite's, else
// `dialects.Scala3`). Each is written verbatim inside the wrapper its kind takes (`wrap`) and parsed by scalac 3.8.4
// (`-Ystop-after:parser`, one run over all), whose first error, if any, is kept with it; a statement or term scalac
// refuses in braces is tried in an indented wrapper (`indented`), and kept so where scalac accepts it. The layout is
// scalameta's under `dialects.Scala3`, the dialect scalafix reads a Scala 3 source with, as support/MetaDump.scala
// prints a tree; a test's own dialect giving another is noted (`#!dialect`); the scanner's stream under it as
// support/RegionStream.scala writes it (`#!regions`). One file a suite:
// `scala-cli run --server=false tests/fix/support/ScalametaCorpus.scala -- <checkout>/tests/.../parsers/dotty tests/fix/scalameta`.
// The probes of the passes over the tree (tests/fix/support/probes, a source each) are a file of their own, Probes.txt,
// a program without a final line break marked `#!no-final-newline` after the format's line break:
// `scala-cli run --server=false tests/fix/support/ScalametaCorpus.scala -- --probes tests/fix/support/probes tests/fix/scalameta/Probes.txt`.
import scala.meta._

object ScalametaCorpus {
  private val parses = Map(
    "stat" -> "Stat", "templStat" -> "Stat", "blockStat" -> "BlockStat", "topStat" -> "TopStat", "term" -> "Term",
    "pat" -> "Pat", "patternTyp" -> "Type", "tpe" -> "Type", "source" -> "Source", "parseCase" -> "Case",
  )
  /** The suites' own helpers over a program: TypeSuite's and PatSuite's. */
  private val asserts = Map("assertTpe" -> "Type", "assertPat" -> "Pat", "assertPatTyp" -> "Type")

  /** A program written as a literal, `.stripMargin` applied where it follows. */
  private def literal(t: Term): Option[String] = t match {
    case Lit.String(v) => Some(v)
    // An interpolation of no argument, `$$` its `$`.
    case Term.Interpolate(Term.Name("s"), List(Lit.String(v)), Nil) => Some(v.replace("$$", "$"))
    case Term.Select(Term.Interpolate(Term.Name("s"), List(Lit.String(v)), Nil), Term.Name("stripMargin")) => Some(v.replace("$$", "$").stripMargin)
    case Term.Select(Lit.String(v), Term.Name("stripMargin")) => Some(v.stripMargin)
    case Term.Apply.After_4_6_0(Term.Select(Lit.String(v), Term.Name("stripMargin")), Term.ArgClause(List(Lit.Char(c)), _)) =>
      Some(v.stripMargin(c))
    case _ => None
  }

  private def dialectIn(t: Tree): Option[String] = t.collect {
    case s @ Term.Select(_, _) if s.syntax.startsWith("dialects.") => s.syntax
    case a @ Term.Apply.After_4_6_0(Term.Select(q, _), _) if q.syntax.startsWith("dialects.") => a.syntax
    case n @ Term.Name(v) if v.startsWith("Scala3") => "dialects." + v
  }.sortBy(-_.length).headOption

  /** The `implicit val dialect` (or a parser for statements) a template or block defines. */
  private def implicits(stats: List[Stat]): (Option[String], Option[String]) = {
    val dialect = stats.collectFirst {
      case Defn.Val(mods, List(Pat.Var(Term.Name("dialect"))), _, rhs) if mods.exists(_.is[Mod.Implicit]) => rhs.syntax
    }
    val parser = stats.collectFirst {
      case d: Defn if d.syntax.contains("blockStat") && d.syntax.contains("Stat") && d.syntax.contains("implicit") => "BlockStat"
      case d: Defn if d.syntax.contains("templStat") && d.syntax.contains("Stat") && d.syntax.contains("implicit") => "Stat"
    }
    (dialect, parser)
  }

  final case class Program(suite: String, test: String, line: Int, kind: String, dialect: String, program: String)

  /** The program inside the wrapper its kind takes, verbatim from a line of its own: a source as written (ended by a
    * line break, as the corpus file holds it), a template statement in an object's body, a block statement in a
    * method's block, a term as a block value, a type as an alias's, a pattern as a case's, a case in a match. Both
    * parsers read the same text. */
  def wrap(p: Program, n: Int): String = p.kind match {
    case "Source" | "TopStat" => if (p.program.endsWith("\n")) p.program else p.program + "\n"
    case "Stat" => s"object W$n {\n${p.program}\n}\n"
    case "BlockStat" => s"object W$n {\n  def w = {\n${p.program}\n}\n}\n"
    case "Term" => s"object W$n {\n  val w = {\n${p.program}\n}\n}\n"
    case "Type" => s"object W$n {\n  type T = ${p.program}\n}\n"
    case "Pat" => s"object W$n {\n  def w(x: Any) = x match {\ncase ${p.program} => ()\n}\n}\n"
    case "Case" => s"object W$n {\n  def w(x: Any) = x match {\n${p.program}\n}\n}\n"
    case other => sys.error(s"no wrapper for $other")
  }

  /** A statement's or term's wrapper under significant indentation, its lines shifted. */
  def indented(p: Program, n: Int): Option[String] = {
    def shift(by: Int) = p.program.linesIterator.map(l => if (l.isEmpty) l else " " * by + l).mkString("\n")
    p.kind match {
      case "Stat" => Some(s"object W$n:\n${shift(2)}\n")
      case "BlockStat" => Some(s"object W$n:\n  def w =\n${shift(4)}\n")
      case "Term" => Some(s"object W$n:\n  val w =\n${shift(4)}\n")
      case _ => None
    }
  }

  /** `dialects.<Name>` and its `.withAllow<X>(<bool>)`s, by reflection over scalameta's dialects. */
  def dialectOf(text: String): Dialect = {
    val Base = """dialects\.(\w+)(.*)""".r
    val With = """\.(with\w+)\((true|false)\)""".r
    text match {
      case Base(name, rest) =>
        val pkg = Class.forName("scala.meta.dialects.package$").getField("MODULE$").get(null)
        val base = pkg.getClass.getMethod(name).invoke(pkg).asInstanceOf[Dialect]
        With.findAllMatchIn(rest).foldLeft(base) { (d, m) =>
          classOf[Dialect].getMethod(m.group(1), classOf[Boolean]).invoke(d, java.lang.Boolean.valueOf(m.group(2).toBoolean)).asInstanceOf[Dialect]
        }
      case _ => sys.error(s"no dialect $text")
    }
  }

  /** The tree as support/MetaDump.scala prints it, or why scalameta refuses the text. */
  def layout(text: String, dialect: Dialect): String =
    dialect(Input.VirtualFile("W.scala", text)).parse[Source] match {
      case Parsed.Success(tree) =>
        val out = new StringBuilder
        def walk(t: Tree, depth: Int): Unit = {
          out ++= "  " * depth + t.productPrefix + " " + t.pos.start + " " + t.pos.end + "\n"
          t.children.foreach(walk(_, depth + 1))
        }
        walk(tree, 0)
        out.result()
      case e: Parsed.Error => "refused: " + e.message.linesIterator.next() + "\n"
    }

  /** scalac 3.8.4's parser over every file at once: each file's first error. */
  def scalac(files: Seq[java.io.File]): Map[String, String] = {
    val reporter = new dotty.tools.dotc.reporting.StoreReporter()
    dotty.tools.dotc.Main.process(Array("-Ystop-after:parser", "-usejavacp", "-color:never") ++ files.map(_.getPath), reporter)
    reporter.allErrors.groupBy(_.pos.source.file.path).map { case (f, ds) => f -> ds.minBy(_.pos.start).message.linesIterator.next() }
  }

  /** Whether scalac 3.8.4's parser accepts the file, a run of its own (its parser crashes on some refused texts). */
  def accepts(file: java.io.File): Boolean = {
    val reporter = new dotty.tools.dotc.reporting.StoreReporter()
    try {
      dotty.tools.dotc.Main.process(Array("-Ystop-after:parser", "-usejavacp", "-color:never", file.getPath), reporter)
      !reporter.hasErrors
    } catch { case _: Throwable => false }
  }

  /** The probes under `dir`, each a source as written, scalac's parse and scalameta's layout of it, to `out`. */
  def probes(dir: String, out: String): Unit = {
    val files = new java.io.File(dir).getAbsoluteFile.listFiles().filter(_.getName.endsWith(".scala")).sortBy(_.getName).toVector
    val refused = scalac(files)
    val b = new StringBuilder
    b ++= "# The probes of the passes over teq fix's tree (tests/fix/support/probes), each a source as written, scalac\n"
    b ++= "# 3.8.4's parse and scalameta 4.17.4's layout of it (support/ScalametaCorpus.scala --probes).\n"
    for ((f, n) <- files.zipWithIndex) {
      val text = new String(java.nio.file.Files.readAllBytes(f.toPath), "UTF-8")
      b ++= s"#!program $n line 1 Source dialects.Scala3 :: ${f.getName.stripSuffix(".scala")}\n"
      b ++= text
      // A program without a final line break: the format's own line break, then the mark of it.
      if (!text.endsWith("\n")) b ++= "\n#!no-final-newline\n"
      b ++= s"#!scalac ${refused.get(f.getPath).fold("ok")("refuses: " + _)}\n"
      b ++= s"#!regions ${scala.meta.internal.parsers.RegionStream.line(text, dialects.Scala3)}\n"
      b ++= "#!layout\n"
      b ++= layout(text, dialects.Scala3)
    }
    java.nio.file.Files.writeString(new java.io.File(out).toPath, b.result())
  }

  def main(args: Array[String]): Unit = {
    if (args.headOption.contains("--probes")) return probes(args(1), args(2))
    val Array(checkout, out) = args
    val suites = new java.io.File(checkout).listFiles().filter(_.getName.endsWith(".scala")).map(_.getPath).sorted
    val found = scala.collection.mutable.ArrayBuffer.empty[Program]
    var unresolved = 0
    var count = 0
    for (path <- suites) {
      val file = new java.io.File(path)
      val suite = file.getName.stripSuffix(".scala")
      val text = new String(java.nio.file.Files.readAllBytes(file.toPath), "UTF-8")
      val tree = dialects.Scala213(text).parse[Source].get
      val (suiteDialect, suiteParser) = tree.collect { case t: Template => implicits(t.body.stats) }
        .find { case (d, p) => d.isDefined || p.isDefined }.getOrElse((None, None))
      def testName(t: Tree): String = t.parent match {
        case Some(Term.Apply.After_4_6_0(Term.Apply.After_4_6_0(Term.Name("test"), Term.ArgClause(name :: _, _)), _)) =>
          name match { case Lit.String(v) => v; case other => other.syntax }
        case Some(p) => testName(p)
        case None => ""
      }
      /** The nearest `implicit val dialect` before `t` in a block or template enclosing it. */
      def blockDialect(t: Tree): Option[String] = t.parent match {
        case Some(b: Term.Block) =>
          implicits(b.stats.takeWhile(s => s.pos.start < t.pos.start))._1.orElse(blockDialect(b))
        case Some(p) => blockDialect(p)
        case None => None
      }
      /** A program named by a `val` before `t` in a block or template enclosing it, or passed as a named argument. */
      def resolve(code: Term, t: Tree): Option[String] = code match {
        case Term.Assign(_, rhs) => resolve(rhs, t)
        case Term.Name(n) =>
          def defined(stats: List[Stat]) = stats.reverse.collectFirst {
            case Defn.Val(_, List(Pat.Var(Term.Name(`n`))), _, rhs) if literal(rhs).isDefined => literal(rhs).get
          }
          def up(x: Tree): Option[String] = x.parent match {
            case Some(b: Term.Block) => defined(b.stats.takeWhile(_.pos.start < t.pos.start)).orElse(up(b))
            case Some(b: Template.Body) => defined(b.stats).orElse(up(b))
            case Some(tm: Template) => defined(tm.body.stats).orElse(up(tm))
            case Some(p) => up(p)
            case None => None
          }
          up(t)
        case other => literal(other)
      }
      def emit(call: Tree, kind: String, code: Term, explicit: Option[String]): Unit = resolve(code, call) match {
        case Some(program) =>
          count += 1
          val dialect = explicit.orElse(blockDialect(call)).orElse(suiteDialect).getOrElse("dialects.Scala3").filterNot(_.isWhitespace)
          val k = if (kind == "Stat" && suiteParser.contains("BlockStat")) "BlockStat" else kind
          found += Program(suite, testName(call), call.pos.startLine + 1, k, dialect, program)
        case None => unresolved += 1; System.err.println(s"unresolved: $suite line ${call.pos.startLine + 1}: ${code.syntax.take(80)}")
      }
      /** The argument lists applied after `call` (`f(code)(tree)(parser, dialect)`), for an explicit dialect. */
      def outer(call: Tree): Tree = call.parent match {
        case Some(p @ Term.Apply.After_4_6_0(fun, _)) if fun eq call => outer(p)
        case _ => call
      }
      def explicitDialect(call: Tree): Option[String] = {
        val top = outer(call)
        if (top eq call) None else dialectIn(top).filter(d => !call.syntax.contains(d) || top.syntax.length > call.syntax.length)
      }
      tree.traverse {
        case call @ Term.Apply.After_4_6_0(Term.ApplyType.After_4_6_0(Term.Name(fn @ ("runTestAssert" | "parseAndCheckTree")), Type.ArgClause(List(t))), Term.ArgClause(code :: _, _)) =>
          emit(call, t.syntax, code, explicitDialect(call))
        case call @ Term.Apply.After_4_6_0(Term.Name("runTestAssert" | "parseAndCheckTree"), Term.ArgClause(code :: _, _)) =>
          emit(call, "Stat", code, explicitDialect(call))
        case call @ Term.Apply.After_4_6_0(Term.Name("checkStat"), Term.ArgClause(code :: _, _)) =>
          emit(call, "Stat", code, explicitDialect(call))
        case call @ Term.Apply.After_4_6_0(Term.ApplyType.After_4_6_0(Term.Name("matchSubStructure"), Type.ArgClause(List(t))), Term.ArgClause(code :: _, _)) =>
          emit(call, t.syntax, code, explicitDialect(call))
        case call @ Term.Apply.After_4_6_0(Term.Name(fn), Term.ArgClause(code :: Nil, _)) if asserts.contains(fn) =>
          emit(call, asserts(fn), code, explicitDialect(call))
        case call @ Term.Apply.After_4_6_0(Term.Name(fn), Term.ArgClause(List(code), _)) if parses.contains(fn) && literal(code).isDefined =>
          // A parse inside a runTestError or an interception asserts an error, not a tree.
          val inError = Iterator.iterate(call.parent)(_.flatMap(_.parent)).takeWhile(_.isDefined).flatten.exists {
            case Term.Apply.After_4_6_0(Term.Name(n), _) => n.startsWith("intercept") || n.contains("Error")
            case Term.Apply.After_4_6_0(Term.ApplyType.After_4_6_0(Term.Name(n), _), _) => n.startsWith("intercept") || n.contains("Error")
            case _ => false
          }
          if (!inError) emit(call, parses(fn), code, explicitDialect(call))
      }
    }
    System.err.println(s"programs $count, unresolved $unresolved")
    val programs = found.toVector
    val dir = java.nio.file.Files.createTempDirectory("scalameta-corpus").toFile
    val files = programs.zipWithIndex.map { case (p, n) =>
      val f = new java.io.File(dir, s"${p.suite}_$n.scala")
      java.nio.file.Files.writeString(f.toPath, wrap(p, n))
      f
    }
    val braced = scalac(files)
    val texts = programs.zipWithIndex.map { case (p, n) =>
      val alternative = if (braced.contains(files(n).getPath)) indented(p, n) else None
      alternative.filter { text =>
        val f = new java.io.File(dir, s"${p.suite}_${n}_indented.scala")
        java.nio.file.Files.writeString(f.toPath, text)
        accepts(f)
      }.getOrElse(wrap(p, n))
    }
    val refused = programs.indices.flatMap(n => braced.get(files(n).getPath).filter(_ => texts(n) == wrap(programs(n), n)).map(n -> _)).toMap
    var otherTree = 0
    new java.io.File(out).mkdirs()
    for ((suite, entries) <- programs.zipWithIndex.groupBy(_._1.suite).toVector.sortBy(_._1)) {
      val b = new StringBuilder
      b ++= s"# The programs of scalameta 4.17.4's $suite (tests/shared/src/test/scala/scala/meta/tests/parsers/dotty, BSD\n"
      b ++= "# 3-Clause, teq's NOTICE), each wrapped, scalameta's layout of it and scalac 3.8.4's parse (support/ScalametaCorpus.scala).\n"
      for ((p, n) <- entries) {
        val text = texts(n)
        b ++= s"#!program $n line ${p.line} ${p.kind} ${p.dialect} :: ${p.test.replace('\n', ' ')}\n"
        b ++= text
        if (!text.endsWith("\n")) b ++= "\n"
        b ++= s"#!scalac ${refused.get(n).fold("ok")("refuses: " + _)}\n"
        val scala3 = layout(text, dialects.Scala3)
        if (p.dialect != "dialects.Scala3" && layout(text, dialectOf(p.dialect)) != scala3) {
          otherTree += 1
          b ++= s"#!dialect ${p.dialect} gives another tree\n"
        }
        b ++= s"#!regions ${scala.meta.internal.parsers.RegionStream.line(text, dialects.Scala3)}\n"
        b ++= "#!layout\n"
        b ++= scala3
      }
      java.nio.file.Files.writeString(new java.io.File(out, s"$suite.txt").toPath, b.result())
    }
    System.err.println(s"scalac refuses ${refused.size} (${braced.size} in braces), another tree under the test's dialect $otherTree")
  }
}
