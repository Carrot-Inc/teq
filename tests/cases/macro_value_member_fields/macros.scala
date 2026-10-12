import scala.quoted.*

// A member lookup on a value's symbol reads the members of its type (dotty's lookupPrefix, the
// value's namedType): the fields of a class parameter, private ones among them, and their trees.
object Probe:
  inline def fields[T](inline x: T): String = ${ fieldsImpl('x) }
  def fieldsImpl[T: Type](x: Expr[T])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val term = x.asTerm match
      case Inlined(_, _, t) => t
      case t => t
    val sym = term.symbol
    val fields = sym.memberFields.map(_.name).sorted.mkString(",")
    val trees = sym.memberFields.map(_.tree).collect { case v: ValDef => v.name + ":" + v.tpt.tpe.show }.sorted.mkString(",")
    val count = sym.fieldMember("count").name
    Expr(s"${sym.name}: $fields; $trees; $count")
