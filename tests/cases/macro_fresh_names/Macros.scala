package fresh

import scala.jdk.CollectionConverters.*
import scala.quoted.*

// A macro that binds its results to locals it names itself, and whose implementation runs the
// collection methods the interpreter has natives for: what it emits must not depend on how the
// standard library ran.
object Stats:
  inline def summary(inline words: String*): String = ${ summaryImpl('words) }

  def summaryImpl(words: Expr[Seq[String]])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val values = words match
      case Varargs(items) => items.map(_.valueOrAbort).toList
      case _ => report.errorAndAbort("literal words expected")
    val distinct = values.distinctBy(_.toLowerCase)
    val byKey = values.groupBy(w => w.length :: w.take(1).toList.map(_.toString)).toList.sortBy(_._1.mkString(","))
    val roots = values.map(w => (0 until w.length).filter(i => i > 0 && w.charAt(i) == '-').map(w.take))
    val list = new java.util.ArrayList[String]()
    values.foreach(list.add)
    val copied = list.asScala.toList
    val sizes = byKey.map((k, ws) => s"${k.mkString("/")}=${ws.size}")
    val text = s"${distinct.mkString(" ")} | ${sizes.mkString(" ")} | ${roots.flatten.mkString(" ")} | ${copied.size}"
    val count = Symbol.newVal(Symbol.spliceOwner, Symbol.freshName("count"), TypeRepr.of[Int], Flags.EmptyFlags, Symbol.noSymbol)
    val label = Symbol.newVal(Symbol.spliceOwner, Symbol.freshName("label"), TypeRepr.of[String], Flags.EmptyFlags, Symbol.noSymbol)
    Block(
      List(ValDef(count, Some(Literal(IntConstant(values.size)))), ValDef(label, Some(Literal(StringConstant(text))))),
      '{ ${ Ref(label).asExprOf[String] } + " #" + ${ Ref(count).asExprOf[Int] } }.asTerm
    ).asExprOf[String]
