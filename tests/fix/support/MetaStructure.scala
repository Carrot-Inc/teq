//> using scala 2.13.16
//> using dep org.scalameta::scalameta:4.17.4
// scalameta 4.17.4's structure of a file, as DumpStructure.scala prints teq's: a node a line, its field, its class, its
// scalar fields and its syntax, each derived accessor's answer under it (std/fix/tree.scala's, `derived`, called on
// scalameta's class), then each child. A case's `structure.txt` is
// `scala-cli run --server=false tests/fix/support/MetaStructure.scala -- <case>/input.scala` (each path given its own,
// beside it).
import scala.meta._

object MetaStructure {
  // A parsed tree's `syntax` is its text where the dialect it is printed in is the one it was parsed in.
  implicit val dialect: Dialect = dialects.Scala3

  // Every derived accessor of std/fix/tree.scala, by class (DumpStructure.scala's `derived`).
  val derived: Map[String, List[String]] = Map(
    "Decl.Def" -> List("paramClauses", "paramss", "tparams"),
    "Decl.Given" -> List("paramClauses", "tparams"),
    "Decl.Type" -> List("tparams"),
    "Defn.Class" -> List("tparams"),
    "Defn.Def" -> List("paramClauses", "paramss", "tparams"),
    "Defn.Enum" -> List("tparams"),
    "Defn.EnumCase" -> List("tparams"),
    "Defn.Given" -> List("paramClauses", "tparams"),
    "Defn.GivenAlias" -> List("paramClauses", "tparams"),
    "Defn.Trait" -> List("tparams"),
    "Defn.Type" -> List("tparams"),
    "Defn.Var" -> List("rhs"),
    "Init" -> List("argss"),
    "Lit.Double" -> List("value"),
    "Lit.Float" -> List("value"),
    "Lit.Null" -> List("value"),
    "Lit.Unit" -> List("value"),
    "Lit.WithUnary" -> List("value"),
    "Name.Anonymous" -> List("value"),
    "Name.Placeholder" -> List("value"),
    "Name.This" -> List("value"),
    "Pat.Extract" -> List("args"),
    "Pat.ExtractInfix" -> List("rhs"),
    "Pkg" -> List("stats"),
    "Template" -> List("self", "stats"),
    "Term.Apply" -> List("args"),
    "Term.ApplyInfix" -> List("args", "targs"),
    "Term.ApplyType" -> List("targs"),
    "Term.ContextFunction" -> List("params"),
    "Term.For" -> List("enums"),
    "Term.ForYield" -> List("enums"),
    "Term.Function" -> List("params"),
    "Term.Match" -> List("cases"),
    "Term.PolyFunction" -> List("tparams"),
    "Term.SelectMatch" -> List("cases"),
    "Term.Try" -> List("catchp"),
    "Type.Apply" -> List("args"),
    "Type.Lambda" -> List("tparams"),
    "Type.Match" -> List("cases"),
    "Type.Param" -> List("tparams"),
    "Type.PolyFunction" -> List("tparams"),
    "Type.Refine" -> List("stats")
  )

  def quoted(s: String): String = {
    val b = new StringBuilder("\"")
    s.foreach {
      case '"' => b ++= "\\\""
      case '\\' => b ++= "\\\\"
      case '\n' => b ++= "\\n"
      case '\r' => b ++= "\\r"
      case '\t' => b ++= "\\t"
      case c if c < ' ' => b ++= "\\u%04x".format(c.toInt)
      case c => b += c
    }
    b += '"'
    b.toString
  }

  def scalar(v: Any): String = v match {
    case s: String => quoted(s)
    case c: Char => quoted(c.toString)
    case s: Symbol => quoted(s.name)
    case x => String.valueOf(x)
  }

  def render(v: Any): String = v match {
    case t: Tree => t.productPrefix + " " + quoted(t.syntax)
    case l: Seq[_] => l.map(render).mkString("[", ", ", "]")
    case None => "None"
    case Some(x) => "Some(" + render(x) + ")"
    case (a, b) => "(" + render(a) + ", " + render(b) + ")"
    case () => "()"
    case null => "null"
    case x => scalar(x)
  }

  def lines(t: Tree, field: String, depth: Int, out: StringBuilder): Unit = {
    val fields = t.productFields.zip(t.productIterator.toList)
    val scalars = fields.collect { case (n, v) if !v.isInstanceOf[Tree] && !v.isInstanceOf[Seq[_]] && !v.isInstanceOf[Option[_]] => n + "=" + scalar(v) }
    out ++= "  " * depth + field + " " + t.productPrefix + scalars.map(" " + _).mkString + " syntax=" + quoted(t.syntax) + "\n"
    val pad = "  " * (depth + 1)
    derived.getOrElse(t.productPrefix, Nil).foreach { m =>
      val answer = t.getClass.getMethods.find(x => x.getName == m && x.getParameterCount == 0).map(_.invoke(t))
      out ++= pad + "." + m + " = " + answer.map(render).getOrElse("<absent>") + "\n"
    }
    fields.foreach {
      case (n, c: Tree) => lines(c, n, depth + 1, out)
      case (n, s: Seq[_]) if s.isEmpty => out ++= pad + n + " Nil\n"
      case (n, s: Seq[_]) => s.foreach {
          case c: Tree => lines(c, n, depth + 1, out)
          case l: Seq[_] => l.foreach { case c: Tree => lines(c, n, depth + 1, out) }
        }
      case (n, None) => out ++= pad + n + " None\n"
      case (n, Some(c: Tree)) => lines(c, n, depth + 1, out)
      case _ =>
    }
  }

  def main(args: Array[String]): Unit = {
    args.foreach { path =>
      val text = new String(java.nio.file.Files.readAllBytes(java.nio.file.Paths.get(path)), "UTF-8")
      val out = new StringBuilder
      Input.VirtualFile(path, text).parse[Source] match {
        case Parsed.Success(tree) => lines(tree, "-", 0, out)
        case e: Parsed.Error => out ++= "PARSE ERROR " + e.pos.startLine + ":" + e.pos.startColumn + " " + e.message + "\n"
      }
      val target = java.nio.file.Paths.get(path).resolveSibling("structure.txt")
      java.nio.file.Files.write(target, out.toString.getBytes("UTF-8"))
    }
  }
}
