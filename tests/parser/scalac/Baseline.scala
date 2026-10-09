//> using scala 3.8.4
//> using dep org.scala-lang::scala3-compiler:3.8.4

// The scalac side of the mutation corpus's baseline (tests/parser/run.py --scalac) and of the
// fixtures (run.py --fixtures --record-scalac): for each line `<id>\t<files>\t<scalac flags>` of
// the list given, the files separated by spaces, scalac 3.8.4's complete diagnostics on them,
// compiled together in this JVM as `scala-cli compile` would (its default reporter's filters,
// every phase), and for a single file the definitions of scalac's own parse tree of it, with
// their owners. One JSON line per entry on stdout:
// {"id":..,"diagnostics":[{"severity","file","line","col","message"}],"defs":[{"kind","name","owner","at"}]}
// with 1-based lines and columns, `file` the diagnostic's file name, `at` the name's offset in
// characters.

import dotty.tools.dotc.Driver
import dotty.tools.dotc.ast.untpd
import dotty.tools.dotc.core.Contexts.*
import dotty.tools.dotc.core.Flags
import dotty.tools.dotc.parsing.Parsers
import dotty.tools.dotc.reporting.*
import dotty.tools.dotc.util.SourceFile
import dotty.tools.io.AbstractFile
import scala.collection.mutable.ListBuffer

class Collect extends AbstractReporter:
  val items = ListBuffer[Diagnostic]()
  def doReport(dia: Diagnostic)(using Context): Unit = items += dia

object Baseline:
  def quote(s: String): String =
    val b = StringBuilder("\"")
    s.foreach {
      case '"' => b ++= "\\\""
      case '\\' => b ++= "\\\\"
      case '\n' => b ++= "\\n"
      case '\t' => b ++= "\\t"
      case c if c < ' ' => b ++= f"\\u${c.toInt}%04x"
      case c => b += c
    }
    b += '"'
    b.toString

  def severity(d: Diagnostic): String = d match
    case _: Diagnostic.Error => "error"
    case _: Diagnostic.Warning => "warning"
    case _ => "info"

  /** The definitions of scalac's parse tree of `path`, outside parameter lists. */
  def defs(path: String, text: String)(using Context): List[String] =
    val source = SourceFile.virtual(path, text)
    val ctx1 = ctx.fresh.setReporter(StoreReporter()).setSource(source)
    val tree = Parsers.Parser(source)(using ctx1).parse()
    val out = ListBuffer[String]()
    object Walker extends untpd.UntypedTreeTraverser:
      var owner: List[String] = Nil
      def record(kind: String, name: String, at: Int): Unit =
        out += s"""{"kind":${quote(kind)},"name":${quote(name)},"owner":${quote(owner.reverse.mkString("/"))},"at":$at}"""
      def within(segment: String)(body: => Unit): Unit =
        val saved = owner
        owner = segment :: owner
        body
        owner = saved
      def traverse(t: untpd.Tree)(using Context): Unit = t match
        case md: untpd.ModuleDef =>
          record("object", md.name.toString, md.span.point)
          within(s"object:${md.name}")(traverse(md.impl))
        case td: untpd.TypeDef =>
          val kind = td.rhs match
            case _: untpd.Template if td.mods.is(Flags.Trait) => "trait"
            case _: untpd.Template if td.mods.is(Flags.Enum) && td.mods.is(Flags.Case) => "case"
            case _: untpd.Template if td.mods.is(Flags.Enum) => "enum"
            case _: untpd.Template => "class"
            case _ => "type"
          if !td.mods.is(Flags.Param) then
            record(kind, td.name.toString, td.span.point)
            within(s"$kind:${td.name}")(traverse(td.rhs))
        case dd: untpd.DefDef =>
          if dd.name.toString != "<init>" then
            record("def", dd.name.toString, dd.span.point)
            within(s"def:${dd.name}")(traverse(dd.rhs))
        case vd: untpd.ValDef =>
          if !vd.mods.is(Flags.Param) && !vd.name.isEmpty && vd.name.toString != "_" then
            val kind = if vd.mods.is(Flags.Mutable) then "var" else "val"
            record(kind, vd.name.toString, vd.span.point)
            within(s"$kind:${vd.name}")(traverse(vd.rhs))
          else traverse(vd.rhs)
        case tmpl: untpd.Template =>
          tmpl.parents.foreach(traverse)
          tmpl.body.foreach(traverse)
        case f: untpd.Function =>
          traverse(f.body)
        case _ =>
          traverseChildren(t)
    Walker.traverse(tree)
    out.toList

  def main(args: Array[String]): Unit =
    val list = scala.io.Source.fromFile(args(0)).getLines().toList
    val classes = java.nio.file.Files.createTempDirectory("teq-parser-baseline").toString
    val cp = System.getProperty("java.class.path")
    for line <- list do
      val fields = line.split("\t", -1)
      val (id, paths) = (fields(0), fields(1).split(" ").toList)
      val path = paths.head
      val flags = if fields.length > 2 && fields(2).nonEmpty then fields(2).split(" ").toList else Nil
      val text = String(java.nio.file.Files.readAllBytes(java.nio.file.Paths.get(path)), "UTF-8")
      val reporter = Collect()
      val driverArgs = (List("-color:never", "-classpath", cp, "-d", classes) ++ flags ++ paths).toArray
      try Driver().process(driverArgs, reporter)
      catch case e: Throwable => reporter.items.clear()
      val diags = reporter.items.toList.map { d =>
        val pos = d.pos
        val (l, c) = if pos.exists then (pos.line + 1, pos.column + 1) else (0, 0)
        val file = if pos.exists then pos.source.file.name else ""
        s"""{"severity":"${severity(d)}","file":${quote(file)},"line":$l,"col":$c,"message":${quote(d.message.linesIterator.nextOption().getOrElse(""))}}"""
      }
      val ds =
        given Context = (new dotty.tools.dotc.core.Contexts.ContextBase).initialCtx.fresh
        if paths.size > 1 then Nil else try defs(path, text) catch case e: Throwable => Nil
      println(s"""{"id":${quote(id)},"diagnostics":[${diags.mkString(",")}],"defs":[${ds.mkString(",")}]}""")
      System.out.flush()
